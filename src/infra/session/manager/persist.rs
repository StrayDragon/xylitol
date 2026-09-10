use uuid::Uuid;

use super::SessionManager;
use crate::infra::session::types::*;
use crate::protocol::error::XySessionStoreError;
use crate::protocol::message::now_ms;
use crate::utils::{lock_rwlock_read, lock_rwlock_write};

/// Write `content` to `tmp_path`, sync, then atomically rename over `path`.
async fn write_session_file_atomically(
    path: &std::path::Path,
    tmp_path: &std::path::Path,
    content: &str,
) -> Result<(), XySessionStoreError> {
    use tokio::io::AsyncWriteExt;

    let mut tmp = tokio::fs::File::create(tmp_path)
        .await
        .map_err(|e| XySessionStoreError::io("create session tmp file", e))?;
    tmp.write_all(content.as_bytes())
        .await
        .map_err(|e| XySessionStoreError::io("write session tmp file", e))?;
    tmp.sync_all()
        .await
        .map_err(|e| XySessionStoreError::io("sync session tmp file", e))?;
    tokio::fs::rename(tmp_path, path)
        .await
        .map_err(|e| XySessionStoreError::io("rename session file", e))
}

impl SessionManager {
    pub(super) async fn write_entries_to_disk(
        &self,
        session_id: &str,
        entries: &[SessionEntry],
    ) -> Result<(), XySessionStoreError> {
        let path = self.session_path(session_id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| XySessionStoreError::io("create sessions dir", e))?;
        }

        let mut content = String::new();
        for entry in entries {
            let line = serde_json::to_string(entry).map_err(XySessionStoreError::from)?;
            content.push_str(&line);
            content.push('\n');
        }

        // Crash-atomic replace (s7): a sibling tmp file + rename means a crash
        // can never leave an existing session file truncated.
        let tmp_path = path.with_extension("jsonl.tmp");
        let write_result = write_session_file_atomically(&path, &tmp_path, &content).await;
        if write_result.is_err() {
            let _ = tokio::fs::remove_file(&tmp_path).await;
        }
        write_result
    }

    /// Record the fork cut on the child header created by [`Self::create`] (s23).
    pub(super) fn stamp_header_fork_at_entry_id(&self, session_id: &str, at_entry_id: &str) {
        let stamp = |entries: &mut Vec<SessionEntry>| {
            if let Some(entry) = entries
                .iter_mut()
                .find(|e| matches!(e, SessionEntry::Header(_)))
                && let SessionEntry::Header(h) = entry
            {
                h.fork_at_entry_id = Some(at_entry_id.to_string());
            }
        };
        {
            let mut pending = lock_rwlock_write(&self.pending_store);
            if let Some(entries) = pending.get_mut(session_id) {
                stamp(entries);
            }
        }
        {
            let mut mem = lock_rwlock_write(&self.in_memory_store);
            if let Some(entries) = mem.get_mut(session_id) {
                stamp(entries);
            }
        }
    }

    pub(super) async fn flush_pending_to_disk(
        &self,
        session_id: &str,
    ) -> Result<(), XySessionStoreError> {
        let pending = {
            let mut store = lock_rwlock_write(&self.pending_store);
            store.remove(session_id).ok_or_else(|| {
                XySessionStoreError::validation(format!(
                    "no pending entries for session: {session_id}"
                ))
            })?
        };

        // `append_with_id` may have already created the JSONL (body rows) while the
        // header was still pending. Blind overwrite would clobber those rows.
        if self.session_file_exists(session_id) {
            let path = self.session_path(session_id);
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| XySessionStoreError::io("read session before pending merge", e))?;
            let (disk, _) = crate::protocol::session::parse_session_jsonl_lines(&content);
            let merged = Self::merge_pending_ahead_of_disk(pending, disk);
            self.write_entries_to_disk(session_id, &merged).await
        } else {
            self.write_entries_to_disk(session_id, &pending).await
        }
    }

    /// Prefers on-disk rows when ids collide; keeps a missing session header from pending.
    fn merge_pending_ahead_of_disk(
        pending: Vec<SessionEntry>,
        disk: Vec<SessionEntry>,
    ) -> Vec<SessionEntry> {
        let mut disk_ids = std::collections::HashSet::new();
        let mut has_header = false;
        for entry in &disk {
            if matches!(entry, SessionEntry::Header(_)) {
                has_header = true;
            }
            if let Some(id) = entry.entry_id() {
                disk_ids.insert(id.to_string());
            }
        }

        let mut merged = Vec::with_capacity(pending.len() + disk.len());
        for entry in pending {
            if matches!(entry, SessionEntry::Header(_)) {
                if !has_header {
                    merged.push(entry);
                }
                continue;
            }
            if let Some(id) = entry.entry_id()
                && disk_ids.contains(id)
            {
                continue;
            }
            merged.push(entry);
        }
        merged.extend(disk);
        merged
    }

    fn update_leaf_from_entry(&self, session_id: &str, entry: &SessionEntry) {
        if let Some(new_id) = entry.entry_id() {
            self.set_leaf(session_id, Some(new_id.to_string()));
        }
    }

    /// Create a new session and record the header entry.
    ///
    /// Idempotent: if the session already has a header (pending / in-memory /
    /// on-disk), this is a no-op. If body rows were appended before `create`
    /// (bind-then-`/model` race), a missing header is inserted ahead of them
    /// instead of wiping those rows.
    pub async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent_session: Option<&str>,
    ) -> Result<(), XySessionStoreError> {
        if self.session_has_header(id).await {
            return Ok(());
        }

        let header = SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: SESSION_VERSION,
            id: id.to_string(),
            timestamp: now_ms(),
            cwd: cwd.unwrap_or(".").to_string(),
            parent_session: parent_session.map(String::from),
            fork_at_entry_id: None,
        });

        match &self.backend {
            SessionBackend::Persisted { .. } => {
                if self.session_file_exists(id) {
                    // Corrupt / headerless JSONL: prepend header on disk.
                    let path = self.session_path(id);
                    let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
                        XySessionStoreError::io("read session before header repair", e)
                    })?;
                    let (disk, _) = crate::protocol::session::parse_session_jsonl_lines(&content);
                    let mut merged = Vec::with_capacity(disk.len() + 1);
                    merged.push(header);
                    merged.extend(disk);
                    self.write_entries_to_disk(id, &merged).await?;
                } else {
                    let mut store = lock_rwlock_write(&self.pending_store);
                    let entries = store.entry(id.to_string()).or_default();
                    entries.insert(0, header);
                }
            }
            SessionBackend::InMemory => {
                let mut store = lock_rwlock_write(&self.in_memory_store);
                let entries = store.entry(id.to_string()).or_default();
                entries.insert(0, header);
            }
        }

        // Only reset leaf when this is a brand-new empty session.
        if self.get_leaf(id).is_none() {
            self.set_leaf(id, None);
        }
        Ok(())
    }

    async fn session_has_header(&self, session_id: &str) -> bool {
        match &self.backend {
            SessionBackend::Persisted { .. } => {
                if self.session_file_exists(session_id) {
                    let path = self.session_path(session_id);
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        let (entries, _) =
                            crate::protocol::session::parse_session_jsonl_lines(&content);
                        return entries.iter().any(|e| matches!(e, SessionEntry::Header(_)));
                    }
                    return false;
                }
                lock_rwlock_read(&self.pending_store)
                    .get(session_id)
                    .is_some_and(|entries| {
                        entries.iter().any(|e| matches!(e, SessionEntry::Header(_)))
                    })
            }
            SessionBackend::InMemory => lock_rwlock_read(&self.in_memory_store)
                .get(session_id)
                .is_some_and(|entries| {
                    entries.iter().any(|e| matches!(e, SessionEntry::Header(_)))
                }),
        }
    }

    /// Append an entry to a session.
    /// Automatically generates id and links parent_id from current leaf.
    /// For persisted sessions, writes to the JSONL file.
    /// For in-memory sessions, stores in a Vec.
    pub async fn append(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        let entry_with_ids = self.inject_ids(session_id, entry);

        match &self.backend {
            SessionBackend::Persisted { .. } => {
                if self.session_file_exists(session_id) {
                    let path = self.session_path(session_id);
                    let line = serde_json::to_string(&entry_with_ids)
                        .map_err(XySessionStoreError::from)?;
                    let content = format!("{line}\n");

                    use tokio::io::AsyncWriteExt;
                    let mut file = tokio::fs::OpenOptions::new()
                        .append(true)
                        .open(&path)
                        .await
                        .map_err(|e| XySessionStoreError::io("open for append", e))?;
                    file.write_all(content.as_bytes())
                        .await
                        .map_err(|e| XySessionStoreError::io("write entry", e))?;
                    file.flush()
                        .await
                        .map_err(|e| XySessionStoreError::io("flush entry", e))?;
                } else {
                    let is_assistant =
                        crate::protocol::session::is_assistant_message(&entry_with_ids);
                    {
                        let mut pending = lock_rwlock_write(&self.pending_store);
                        let entries = pending.entry(session_id.to_string()).or_default();
                        // Defense: body rows must never sit in pending without a header
                        // (bind-then-append before `create` / `ensure_session`).
                        if !entries.iter().any(|e| matches!(e, SessionEntry::Header(_))) {
                            entries.insert(
                                0,
                                SessionEntry::Header(SessionHeader {
                                    entry_type: "session".into(),
                                    version: SESSION_VERSION,
                                    id: session_id.to_string(),
                                    timestamp: now_ms(),
                                    cwd: ".".into(),
                                    parent_session: None,
                                    fork_at_entry_id: None,
                                }),
                            );
                        }
                        entries.push(entry_with_ids.clone());
                    }
                    if is_assistant {
                        self.flush_pending_to_disk(session_id).await?;
                    }
                }
            }
            SessionBackend::InMemory => {
                let mut store = lock_rwlock_write(&self.in_memory_store);
                store
                    .entry(session_id.to_string())
                    .or_default()
                    .push(entry_with_ids.clone());
            }
        }

        self.update_leaf_from_entry(session_id, &entry_with_ids);
        Ok(())
    }

    /// Inject auto-generated id and parent_id into an entry.
    fn inject_ids(&self, session_id: &str, entry: &SessionEntry) -> SessionEntry {
        let new_id = Uuid::new_v4().to_string();
        let parent_id = self.get_leaf(session_id);
        let now = now_ms();

        // Create a new entry with injected ids
        Self::clone_entry_with_ids(entry, &new_id, parent_id.as_deref(), now)
    }

    pub(super) fn clone_entry_with_ids(
        entry: &SessionEntry,
        id: &str,
        parent_id: Option<&str>,
        timestamp: u64,
    ) -> SessionEntry {
        let base = EntryBase {
            entry_type: entry.entry_type().to_string(),
            id: id.to_string(),
            parent_id: parent_id.map(String::from),
            timestamp,
        };

        match entry {
            SessionEntry::Header(h) => SessionEntry::Header(SessionHeader {
                entry_type: "session".into(),
                version: SESSION_VERSION,
                id: id.to_string(),
                timestamp,
                cwd: String::new(),
                parent_session: parent_id.map(String::from),
                fork_at_entry_id: h.fork_at_entry_id.clone(),
            }),
            SessionEntry::Message(m) => SessionEntry::Message(MessageEntry {
                base,
                message: m.message.clone(),
            }),
            SessionEntry::Compaction(c) => SessionEntry::Compaction(CompactionEntry {
                base,
                summary: c.summary.clone(),
                first_kept_entry_id: c.first_kept_entry_id.clone(),
                tokens_before: c.tokens_before,
                details: c.details.clone(),
                from_hook: c.from_hook,
            }),
            SessionEntry::BranchSummary(b) => SessionEntry::BranchSummary(BranchSummaryEntry {
                base,
                from_id: b.from_id.clone(),
                summary: b.summary.clone(),
                details: b.details.clone(),
                from_hook: b.from_hook,
            }),
            SessionEntry::ModelChange(mc) => SessionEntry::ModelChange(ModelChangeEntry {
                base,
                provider: mc.provider.clone(),
                model_id: mc.model_id.clone(),
            }),
            SessionEntry::ThinkingLevelChange(tc) => {
                SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
                    base,
                    thinking_level: tc.thinking_level.clone(),
                })
            }
            SessionEntry::Custom(c) => SessionEntry::Custom(CustomEntry {
                base,
                custom_type: c.custom_type.clone(),
                data: c.data.clone(),
            }),
            SessionEntry::CustomMessage(cm) => SessionEntry::CustomMessage(CustomMessageEntry {
                base,
                custom_type: cm.custom_type.clone(),
                content: cm.content.clone(),
                display: cm.display,
                details: cm.details.clone(),
            }),
            SessionEntry::Label(l) => SessionEntry::Label(LabelEntry {
                base,
                target_id: l.target_id.clone(),
                label: l.label.clone(),
            }),
            SessionEntry::SessionInfo(si) => SessionEntry::SessionInfo(SessionInfoEntry {
                base,
                name: si.name.clone(),
            }),
        }
    }

    /// Append an entry with explicit id (for fork operations).
    pub async fn append_with_id(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        // Flush deferred header (and any pending rows) before writing the file
        // directly — otherwise `load` ignores pending once the file exists, and a
        // later `flush_pending_to_disk` can overwrite body rows with header-only.
        let has_pending = lock_rwlock_read(&self.pending_store).contains_key(session_id);
        if has_pending {
            self.flush_pending_to_disk(session_id).await?;
        }

        let path = self.session_path(session_id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| XySessionStoreError::io("create sessions dir", e))?;
        }
        let line = serde_json::to_string(entry).map_err(XySessionStoreError::from)?;
        let content = format!("{line}\n");

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await
            .map_err(|e| XySessionStoreError::io("open for append", e))?;
        file.write_all(content.as_bytes())
            .await
            .map_err(|e| XySessionStoreError::io("write entry", e))?;
        file.flush()
            .await
            .map_err(|e| XySessionStoreError::io("flush entry", e))?;

        if let Some(new_id) = entry.entry_id() {
            self.set_leaf(session_id, Some(new_id.to_string()));
        }

        Ok(())
    }
}
