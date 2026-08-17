use super::SessionManager;
use crate::infra::session::types::*;
use crate::protocol::error::XySessionStoreError;

impl SessionManager {
    /// Load all entries from a session (latest [`SESSION_VERSION`] only).
    /// For persisted sessions, reads from the JSONL file or pending memory.
    /// For in-memory sessions, returns from the in-memory store.
    pub async fn load(&self, session_id: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let entries = match &self.backend {
            SessionBackend::InMemory { .. } => {
                let entries = self
                    .in_memory_store
                    .read()
                    .expect("RwLock not poisoned")
                    .get(session_id)
                    .cloned()
                    .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
                crate::protocol::session::enforce_session_version(&entries)?;
                entries
            }
            SessionBackend::Persisted { .. } => {
                if self.session_file_exists(session_id) {
                    let path = self.session_path(session_id);
                    let content = tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|e| XySessionStoreError::io("read session", e))?;
                    crate::protocol::session::parse_session_jsonl(&content)?
                } else {
                    let entries = self
                        .pending_store
                        .read()
                        .expect("RwLock not poisoned")
                        .get(session_id)
                        .cloned()
                        .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
                    crate::protocol::session::enforce_session_version(&entries)?;
                    entries
                }
            }
        };

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
            let mut pending = self.pending_store.write().expect("RwLock not poisoned");
            pending.remove(session_id);
        }
        {
            let mut leaf = self.leaf_ids.write().expect("RwLock not poisoned");
            leaf.remove(session_id);
        }
        match &self.backend {
            SessionBackend::InMemory { .. } => {
                let mut store = self.in_memory_store.write().expect("RwLock not poisoned");
                store.remove(session_id);
                Ok(())
            }
            SessionBackend::Persisted { .. } => {
                let path = self.session_path(session_id);
                if path.exists() {
                    tokio::fs::remove_file(&path)
                        .await
                        .map_err(|e| XySessionStoreError::io("delete session file", e))?;
                }
                Ok(())
            }
        }
    }

    /// List all session IDs with metadata.
    ///
    /// Includes on-disk `.jsonl` sessions and not-yet-flushed pending sessions
    /// (created / user-only before first assistant flush).
    pub async fn list(&self) -> Result<Vec<String>, XySessionStoreError> {
        let dir = match tokio::fs::read_dir(&self.sessions_dir).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Still surface pending-only sessions below.
                let pending = self.pending_store.read().expect("RwLock not poisoned");
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
            if name_str.ends_with(".jsonl") {
                let id = name_str.trim_end_matches(".jsonl").to_string();
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
            let pending = self.pending_store.read().expect("RwLock not poisoned");
            for id in pending.keys() {
                if !ids.iter().any(|existing| existing == id) {
                    ids.push(id.clone());
                }
            }
        }

        Ok(ids)
    }

    // ── Tree navigation ─────────────────────────────────────────

    /// Get an entry by id.
    pub async fn get_entry(
        &self,
        session_id: &str,
        entry_id: &str,
    ) -> Result<Option<SessionEntry>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        Ok(entries.into_iter().find(|e| e.entry_id() == Some(entry_id)))
    }

    /// Get the current leaf entry.
    pub async fn get_leaf_entry(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionEntry>, XySessionStoreError> {
        let leaf_id = self.get_leaf(session_id);
        match leaf_id {
            Some(id) => self.get_entry(session_id, &id).await,
            None => Ok(None),
        }
    }

    /// Get the current leaf id.
    pub fn get_leaf_id(&self, session_id: &str) -> Option<String> {
        self.get_leaf(session_id)
    }

    /// Get the current session name from the latest session_info entry.
    pub async fn get_session_name(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        Ok(session_display_name_from_entries(&entries))
    }

    /// Load entries for [`crate::protocol::ports::XySessionStore::list_sessions`] without mutating leaf tracking.
    ///
    /// Disk sessions: peek header version first so legacy files skip without a full JSONL parse.
    pub(super) async fn load_entries_for_list(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        use crate::protocol::session::{
            SESSION_VERSION, enforce_session_version, parse_session_jsonl,
            peek_session_header_version,
        };

        match &self.backend {
            SessionBackend::InMemory { .. } => {
                let entries = self
                    .in_memory_store
                    .read()
                    .expect("RwLock not poisoned")
                    .get(session_id)
                    .cloned()
                    .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
                enforce_session_version(&entries)?;
                Ok(entries)
            }
            SessionBackend::Persisted { .. } => {
                if self.session_file_exists(session_id) {
                    let path = self.session_path(session_id);
                    let content = tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|e| XySessionStoreError::io("read session", e))?;
                    if let Some(v) = peek_session_header_version(&content)
                        && v != SESSION_VERSION
                    {
                        return Err(XySessionStoreError::validation(format!(
                            "session header version {v} is not supported (require {SESSION_VERSION}); refusing legacy migrate"
                        )));
                    }
                    parse_session_jsonl(&content)
                } else {
                    let entries = self
                        .pending_store
                        .read()
                        .expect("RwLock not poisoned")
                        .get(session_id)
                        .cloned()
                        .ok_or_else(|| XySessionStoreError::not_found(session_id))?;
                    enforce_session_version(&entries)?;
                    Ok(entries)
                }
            }
        }
    }
}

/// Latest non-empty `session_info.name` (same rule as [`SessionManager::get_session_name`]).
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
