use super::SessionManager;
use crate::infra::session::types::*;
use crate::protocol::error::XySessionStoreError;
use crate::utils::{lock_rwlock_read, lock_rwlock_write};

impl SessionManager {
    /// Load only the segments needed to resolve the requested leaf ancestry.
    ///
    /// The active segment is always hot. Sealed segments are opened from newest
    /// to oldest only when a missing parent id requires them; full logical
    /// loading remains available through [`Self::load`]. When the current
    /// branch reaches a compaction, ancestry stops at its first kept entry.
    pub(super) async fn load_branch_entries(
        &self,
        session_id: &str,
        leaf_hint: Option<&str>,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        if matches!(&self.backend, SessionBackend::InMemory) {
            return self.read_entries(session_id).await;
        }

        if self.legacy_session_path(session_id).exists() && !self.manifest_path(session_id).exists()
        {
            self.migrate_legacy_session(session_id).await?;
        }
        if !self.manifest_path(session_id).exists() {
            return self.read_entries(session_id).await;
        }
        let manifest = self.read_manifest(session_id).await?;
        let active_entries = self
            .read_segment(session_id, &manifest.active_segment)
            .await?;

        let mut leaf = leaf_hint
            .map(str::to_owned)
            .or_else(|| self.get_leaf(session_id))
            .or_else(|| crate::protocol::session::transcript_leaf_anchor(&active_entries, None));
        if leaf.is_none() {
            leaf = manifest.leaf_entry_id.clone();
        }
        let resolved_leaf = leaf.clone();
        let mut sealed_entries: Vec<Option<Vec<SessionEntry>>> =
            vec![None; manifest.sealed_segments.len()];
        let mut visited = std::collections::HashSet::new();
        let mut compaction_cut = None;
        let mut stopped_at_compaction_cut = false;

        while let Some(current_id) = leaf.clone() {
            if !visited.insert(current_id.clone()) {
                break;
            }
            let current = active_entries
                .iter()
                .find(|entry| entry.entry_id() == Some(current_id.as_str()))
                .cloned()
                .or_else(|| {
                    sealed_entries.iter().flatten().find_map(|entries| {
                        entries
                            .iter()
                            .find(|entry| entry.entry_id() == Some(current_id.as_str()))
                            .cloned()
                    })
                });

            let current = if let Some(current) = current {
                current
            } else {
                let mut found = None;
                for index in (0..manifest.sealed_segments.len()).rev() {
                    if sealed_entries[index].is_none() {
                        sealed_entries[index] = Some(
                            self.read_segment(session_id, &manifest.sealed_segments[index])
                                .await?,
                        );
                    }
                    if let Some(entry) = sealed_entries[index].as_ref().and_then(|entries| {
                        entries
                            .iter()
                            .find(|entry| entry.entry_id() == Some(current_id.as_str()))
                    }) {
                        found = Some(entry.clone());
                        break;
                    }
                }
                let Some(current) = found else {
                    break;
                };
                current
            };
            if let SessionEntry::Compaction(compaction) = &current {
                compaction_cut = Some(compaction.first_kept_entry_id.clone());
            }
            let reached_compaction_cut = compaction_cut.as_deref() == Some(current_id.as_str());
            leaf = current.parent_id().map(str::to_owned);
            if reached_compaction_cut {
                stopped_at_compaction_cut = true;
                break;
            }
        }

        let mut entries = Vec::new();
        for segment_entries in sealed_entries.into_iter().flatten() {
            entries.extend(segment_entries);
        }
        entries.extend(active_entries);
        if stopped_at_compaction_cut
            && let Some(cut_id) = compaction_cut
            && let Some(cut_index) = entries
                .iter()
                .position(|entry| entry.entry_id() == Some(cut_id.as_str()))
        {
            entries.drain(..cut_index);
        }
        self.set_leaf(session_id, resolved_leaf);
        Ok(entries)
    }

    /// Read all entries without changing the current leaf.
    pub(super) async fn read_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let entries = match &self.backend {
            SessionBackend::InMemory => {
                let entries = lock_rwlock_read(&self.in_memory_store)
                    .get(session_id)
                    .cloned()
                    .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
                crate::protocol::session::enforce_session_version(&entries)?;
                entries
            }
            SessionBackend::Persisted { .. } => {
                if self.manifest_path(session_id).exists() {
                    let manifest = self.read_manifest(session_id).await?;
                    self.load_entries(session_id, &manifest).await?
                } else if self.legacy_session_path(session_id).exists() {
                    self.migrate_legacy_session(session_id).await?;
                    let manifest = self.read_manifest(session_id).await?;
                    self.load_entries(session_id, &manifest).await?
                } else {
                    let entries = lock_rwlock_read(&self.pending_store)
                        .get(session_id)
                        .cloned()
                        .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
                    crate::protocol::session::enforce_session_version(&entries)?;
                    entries
                }
            }
        };
        Ok(entries)
    }

    /// Load all entries from a session (latest [`SESSION_VERSION`] only).
    /// For persisted sessions, reads from the JSONL file or pending memory.
    /// For in-memory sessions, returns from the in-memory store and updates
    /// the leaf to the final logical entry.
    pub async fn load(&self, session_id: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let entries = self.read_entries(session_id).await?;
        if let Some(last) = entries.last() {
            if let Some(id) = last.entry_id() {
                self.set_leaf(session_id, Some(id.to_string()));
            }
        } else {
            self.set_leaf(session_id, None);
        }

        Ok(entries)
    }

    /// Load and validate that the session's CWD exists.
    ///
    /// If the CWD from the session header does not exist, tries `fallback_cwd`.
    /// Returns an error if neither directory is accessible.
    ///
    /// Spec @executable contract (`resume_session` BDD steps); no product caller
    /// today — product resume goes through `XySessionStore::load_entries`.
    #[allow(dead_code)] // spec @executable contract, not product-called
    pub async fn load_validated(
        &self,
        session_id: &str,
        fallback_cwd: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        assert_session_cwd_exists(&entries, fallback_cwd)?;
        Ok(entries)
    }

    /// Delete a session file and in-memory tracking (c1065 resume panel).
    pub async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        {
            let mut pending = lock_rwlock_write(&self.pending_store);
            pending.remove(session_id);
        }
        {
            let mut leaf = lock_rwlock_write(&self.leaf_ids);
            leaf.remove(session_id);
        }
        match &self.backend {
            SessionBackend::InMemory => {
                let mut store = lock_rwlock_write(&self.in_memory_store);
                store.remove(session_id);
                Ok(())
            }
            SessionBackend::Persisted { .. } => {
                let session_dir = self.session_dir_path(session_id);
                if session_dir.exists() {
                    tokio::fs::remove_dir_all(&session_dir)
                        .await
                        .map_err(|e| XySessionStoreError::io("delete session directory", e))?;
                }
                let legacy = self.legacy_session_path(session_id);
                if legacy.exists() {
                    tokio::fs::remove_file(&legacy)
                        .await
                        .map_err(|e| XySessionStoreError::io("delete legacy session file", e))?;
                }
                Ok(())
            }
        }
    }

    /// List all session IDs with metadata.
    ///
    /// Includes on-disk session directories, legacy files and not-yet-flushed pending sessions
    /// (created / user-only before first assistant flush).
    pub async fn list(&self) -> Result<Vec<String>, XySessionStoreError> {
        let dir = match tokio::fs::read_dir(&self.sessions_dir).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Still surface pending-only sessions below.
                let pending = lock_rwlock_read(&self.pending_store);
                let mut pending_ids: Vec<_> = pending.keys().cloned().collect();
                pending_ids.sort();
                return Ok(pending_ids);
            }
            Err(e) => return Err(XySessionStoreError::io("read sessions dir", e)),
        };

        let mut entries = Vec::new();
        let mut read = dir;
        loop {
            match read.next_entry().await {
                Ok(Some(entry)) => entries.push(entry),
                Ok(None) => break,
                Err(e) => return Err(XySessionStoreError::io("read dir entry", e)),
            }
        }

        let mut files: Vec<_> = Vec::new();
        for entry in &entries {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if entry
                .file_type()
                .await
                .map(|ty| ty.is_dir())
                .unwrap_or(false)
            {
                let manifest = entry.path().join("manifest.json");
                if manifest.exists() {
                    let modified = entry
                        .metadata()
                        .await
                        .map(|m| m.modified().ok())
                        .ok()
                        .flatten();
                    files.push((name_str.into_owned(), modified));
                }
            } else if name_str.ends_with(".jsonl") {
                let id = name_str.trim_end_matches(".jsonl").to_string();
                if self.session_dir_path(&id).join("manifest.json").exists() {
                    continue;
                }
                let modified = entry
                    .metadata()
                    .await
                    .map(|m| m.modified().ok())
                    .ok()
                    .flatten();
                files.push((id, modified));
            }
        }

        files.sort_by_key(|(_, m)| std::cmp::Reverse(*m));
        let mut ids: Vec<String> = files.into_iter().map(|(id, _)| id).collect();

        if matches!(&self.backend, SessionBackend::Persisted { .. }) {
            let pending = lock_rwlock_read(&self.pending_store);
            for id in pending.keys() {
                if !ids.iter().any(|existing| existing == id) {
                    ids.push(id.clone());
                }
            }
        }

        Ok(ids)
    }

    // ── Tree navigation ─────────────────────────────────────────

    /// Get an entry by id (BDD label bookkeeping path; no product caller).
    #[allow(dead_code)] // BDD @executable label steps, not product-called
    pub async fn get_entry(
        &self,
        session_id: &str,
        entry_id: &str,
    ) -> Result<Option<SessionEntry>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        Ok(entries.into_iter().find(|e| e.entry_id() == Some(entry_id)))
    }

    /// Get the current leaf id.
    pub fn get_leaf_id(&self, session_id: &str) -> Option<String> {
        self.get_leaf(session_id)
    }

    /// Load entries for [`crate::protocol::ports::XySessionStore::list_sessions`] without mutating leaf tracking.
    ///
    /// Disk sessions: peek header version first so legacy files skip without a full JSONL parse.
    pub(super) async fn load_entries_for_list(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        match &self.backend {
            SessionBackend::InMemory => {
                let entries = lock_rwlock_read(&self.in_memory_store)
                    .get(session_id)
                    .cloned()
                    .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
                crate::protocol::session::enforce_session_version(&entries)?;
                Ok(entries)
            }
            SessionBackend::Persisted { .. } => {
                if self.manifest_path(session_id).exists() {
                    let manifest = self.read_manifest(session_id).await?;
                    self.load_entries(session_id, &manifest).await
                } else if self.legacy_session_path(session_id).exists() {
                    self.migrate_legacy_session(session_id).await?;
                    let manifest = self.read_manifest(session_id).await?;
                    self.load_entries(session_id, &manifest).await
                } else {
                    let entries = lock_rwlock_read(&self.pending_store)
                        .get(session_id)
                        .cloned()
                        .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
                    crate::protocol::session::enforce_session_version(&entries)?;
                    Ok(entries)
                }
            }
        }
    }
}

/// Latest non-empty `session_info.name` (same rule as the removed
/// `SessionManager::get_session_name`; drives list/resume display).
pub(super) fn session_display_name_from_entries(entries: &[SessionEntry]) -> Option<String> {
    for entry in entries.iter().rev() {
        if let SessionEntry::SessionInfo(si) = entry {
            return si.name.clone().filter(|n| !n.is_empty());
        }
    }
    None
}

// ── CWD Validation ──────────────────────────────────────────────────

/// Validate that the session's working directory exists.
///
/// Checks the CWD stored in the session header. If the directory does not
/// exist, tries `fallback_cwd`. Returns an error if neither is accessible.
pub fn assert_session_cwd_exists(
    entries: &[SessionEntry],
    fallback_cwd: &str,
) -> Result<(), XySessionStoreError> {
    // Find the session header
    let header = entries
        .iter()
        .find_map(|e| {
            if let SessionEntry::Header(h) = e {
                Some(h)
            } else {
                None
            }
        })
        .ok_or_else(|| XySessionStoreError::validation("session has no header entry"))?;

    let cwd = if header.cwd.is_empty() {
        "."
    } else {
        &header.cwd
    };

    let cwd_path = std::path::Path::new(cwd);
    if cwd_path.is_dir() {
        return Ok(());
    }

    // Try fallback
    let fallback_path = std::path::Path::new(fallback_cwd);
    if fallback_path.is_dir() {
        return Ok(());
    }

    Err(XySessionStoreError::validation(format!(
        "Session working directory '{}' does not exist. Fallback '{}' also not found.",
        cwd, fallback_cwd
    )))
}
