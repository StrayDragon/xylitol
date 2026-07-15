//! SessionManager — JSONL file-based session storage.
//!
//! Handles create, append, load, list, exists, tree navigation,
//! build_session_context, and version migration for sessions.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::types::*;
use crate::domain::lifecycle::XyEvent;
use crate::runtime_protocol::{XyEventSink, XySessionStore};

/// Manages session persistence using JSONL files or in-memory storage.
///
/// [`Clone`] shares interior stores via [`Arc`] so handles remain coherent after
/// deferred (pre-flush) creates — deep-copying pending would drop sessions on
/// `mgr.clone()` + mutate patterns used by tests and thin wrappers.
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions_dir: PathBuf,
    /// Storage backend.
    backend: SessionBackend,
    /// Per-session leaf node tracking (in-memory).
    /// session_id -> current leaf entry id (None = root).
    leaf_ids: Arc<RwLock<HashMap<String, Option<String>>>>,
    /// Active session tracking.
    active_session: Arc<RwLock<Option<String>>>,
    /// In-memory entry storage (used when backend is InMemory).
    in_memory_store: Arc<RwLock<HashMap<String, Vec<SessionEntry>>>>,
    /// Pending entries for persisted sessions not yet flushed to disk.
    pending_store: Arc<RwLock<HashMap<String, Vec<SessionEntry>>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            sessions_dir: PathBuf::from("."),
            backend: SessionBackend::Persisted {
                sessions_dir: PathBuf::from("."),
            },
            leaf_ids: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
            pending_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Parameters for [`SessionManager::append_bash_execution`].
pub struct BashExecutionParams<'a> {
    pub session_id: &'a str,
    pub command: &'a str,
    pub output: &'a str,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub truncated: bool,
    pub full_output_path: Option<&'a str>,
    pub exclude_from_context: bool,
}

impl SessionManager {
    /// Create a new SessionManager with the given sessions directory.
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            sessions_dir: sessions_dir.clone(),
            backend: SessionBackend::Persisted {
                sessions_dir: sessions_dir.clone(),
            },
            leaf_ids: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
            pending_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create an in-memory SessionManager (no disk writes).
    /// All entries are stored in a Vec, suitable for ephemeral sessions.
    pub fn in_memory() -> Self {
        Self {
            sessions_dir: PathBuf::from("."),
            backend: SessionBackend::InMemory {
                entries: Vec::new(),
            },
            leaf_ids: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
            pending_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Default sessions directory: ~/.xylitol/sessions/
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol")
            .join("sessions")
    }

    // ── Leaf tracking ───────────────────────────────────────────

    fn set_leaf(&self, session_id: &str, entry_id: Option<String>) {
        self.leaf_ids
            .write()
            .expect("RwLock not poisoned")
            .insert(session_id.to_string(), entry_id);
    }

    fn get_leaf(&self, session_id: &str) -> Option<String> {
        self.leaf_ids
            .read()
            .expect("RwLock not poisoned")
            .get(session_id)
            .cloned()
            .unwrap_or(None)
    }

    fn session_file_exists(&self, session_id: &str) -> bool {
        matches!(&self.backend, SessionBackend::Persisted { .. })
            && self.session_path(session_id).exists()
    }

    async fn write_entries_to_disk(
        &self,
        session_id: &str,
        entries: &[SessionEntry],
    ) -> Result<(), String> {
        let path = self.session_path(session_id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create sessions dir: {e}"))?;
        }

        let mut content = String::new();
        for entry in entries {
            let line = serde_json::to_string(entry).map_err(|e| format!("serialize entry: {e}"))?;
            content.push_str(&line);
            content.push('\n');
        }

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| format!("write session file: {e}"))?;
        Ok(())
    }

    async fn flush_pending_to_disk(&self, session_id: &str) -> Result<(), String> {
        let entries = {
            let mut pending = self.pending_store.write().expect("RwLock not poisoned");
            pending
                .remove(session_id)
                .ok_or_else(|| format!("no pending entries for session: {session_id}"))?
        };
        self.write_entries_to_disk(session_id, &entries).await
    }

    fn update_leaf_from_entry(&self, session_id: &str, entry: &SessionEntry) {
        if let Some(new_id) = entry.entry_id() {
            self.set_leaf(session_id, Some(new_id.to_string()));
        }
    }

    // ── CRUD ────────────────────────────────────────────────────

    /// Get the file path for a session.
    fn session_path(&self, id: &str) -> PathBuf {
        match &self.backend {
            SessionBackend::Persisted { sessions_dir } => sessions_dir.join(format!("{id}.jsonl")),
            SessionBackend::InMemory { .. } => PathBuf::from("/dev/null"),
        }
    }

    /// Get the session file, if persisted.
    pub fn get_session_file(&self, id: &str) -> Option<PathBuf> {
        match &self.backend {
            SessionBackend::Persisted { .. } => Some(self.session_path(id)),
            SessionBackend::InMemory { .. } => None,
        }
    }

    /// Check if a session exists.
    pub fn exists(&self, id: &str) -> bool {
        match &self.backend {
            SessionBackend::Persisted { .. } => {
                self.session_file_exists(id)
                    || self
                        .pending_store
                        .read()
                        .expect("RwLock not poisoned")
                        .contains_key(id)
            }
            SessionBackend::InMemory { .. } => self
                .in_memory_store
                .read()
                .expect("RwLock not poisoned")
                .contains_key(id),
        }
    }

    /// Create a new session and record the header entry.
    pub async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent_session: Option<&str>,
    ) -> Result<(), String> {
        let header = SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: 4,
            id: id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            cwd: cwd.unwrap_or(".").to_string(),
            parent_session: parent_session.map(String::from),
        });

        match &self.backend {
            SessionBackend::Persisted { .. } => {
                self.pending_store
                    .write()
                    .expect("RwLock not poisoned")
                    .insert(id.to_string(), vec![header]);
            }
            SessionBackend::InMemory { .. } => {
                self.in_memory_store
                    .write()
                    .expect("RwLock not poisoned")
                    .insert(id.to_string(), vec![header]);
            }
        }

        self.set_leaf(id, None);
        Ok(())
    }

    /// Append an entry to a session.
    /// Automatically generates id and links parent_id from current leaf.
    /// For persisted sessions, writes to the JSONL file.
    /// For in-memory sessions, stores in a Vec.
    pub async fn append(&self, session_id: &str, entry: &SessionEntry) -> Result<(), String> {
        let entry_with_ids = self.inject_ids(session_id, entry);

        match &self.backend {
            SessionBackend::Persisted { .. } => {
                if self.session_file_exists(session_id) {
                    let path = self.session_path(session_id);
                    let line = serde_json::to_string(&entry_with_ids)
                        .map_err(|e| format!("serialize entry: {e}"))?;
                    let content = format!("{line}\n");

                    use tokio::io::AsyncWriteExt;
                    let mut file = tokio::fs::OpenOptions::new()
                        .append(true)
                        .open(&path)
                        .await
                        .map_err(|e| format!("open for append: {e}"))?;
                    file.write_all(content.as_bytes())
                        .await
                        .map_err(|e| format!("write entry: {e}"))?;
                } else {
                    let is_assistant =
                        crate::domain::session_types::is_assistant_message(&entry_with_ids);
                    {
                        let mut pending = self.pending_store.write().expect("RwLock not poisoned");
                        pending
                            .entry(session_id.to_string())
                            .or_default()
                            .push(entry_with_ids.clone());
                    }
                    if is_assistant {
                        self.flush_pending_to_disk(session_id).await?;
                    }
                }
            }
            SessionBackend::InMemory { .. } => {
                let mut store = self.in_memory_store.write().expect("RwLock not poisoned");
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
        let now = Utc::now().to_rfc3339();

        // Create a new entry with injected ids
        Self::clone_entry_with_ids(entry, &new_id, parent_id.as_deref(), &now)
    }

    fn clone_entry_with_ids(
        entry: &SessionEntry,
        id: &str,
        parent_id: Option<&str>,
        timestamp: &str,
    ) -> SessionEntry {
        let base = EntryBase {
            entry_type: entry.entry_type().to_string(),
            id: id.to_string(),
            parent_id: parent_id.map(String::from),
            timestamp: timestamp.to_string(),
        };

        match entry {
            SessionEntry::Header(_) => SessionEntry::Header(SessionHeader {
                entry_type: "session".into(),
                version: 4,
                id: id.to_string(),
                timestamp: timestamp.to_string(),
                cwd: String::new(),
                parent_session: parent_id.map(String::from),
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
            SessionEntry::BashExecution(b) => SessionEntry::BashExecution(BashExecutionEntry {
                base,
                command: b.command.clone(),
                output: b.output.clone(),
                exit_code: b.exit_code,
                cancelled: b.cancelled,
                truncated: b.truncated,
                full_output_path: b.full_output_path.clone(),
                exclude_from_context: b.exclude_from_context,
            }),
        }
    }

    /// Append an entry with explicit id (for fork operations).
    pub async fn append_with_id(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), String> {
        let path = self.session_path(session_id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create sessions dir: {e}"))?;
        }
        let line = serde_json::to_string(entry).map_err(|e| format!("serialize entry: {e}"))?;
        let content = format!("{line}\n");

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await
            .map_err(|e| format!("open for append: {e}"))?;
        file.write_all(content.as_bytes())
            .await
            .map_err(|e| format!("write entry: {e}"))?;

        if let Some(new_id) = entry.entry_id() {
            self.set_leaf(session_id, Some(new_id.to_string()));
        }

        Ok(())
    }

    /// Load all entries from a session (with v3→v4 migration if needed).
    /// For persisted sessions, reads from the JSONL file or pending memory.
    /// For in-memory sessions, returns from the in-memory store.
    pub async fn load(&self, session_id: &str) -> Result<Vec<SessionEntry>, String> {
        let mut entries = match &self.backend {
            SessionBackend::InMemory { .. } => self
                .in_memory_store
                .read()
                .expect("RwLock not poisoned")
                .get(session_id)
                .cloned()
                .ok_or_else(|| format!("session not found: {session_id}"))?,
            SessionBackend::Persisted { .. } => {
                if self.session_file_exists(session_id) {
                    let path = self.session_path(session_id);
                    let content = tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|e| format!("read session: {e}"))?;
                    Self::parse_entries_from_content(&content)?
                } else {
                    self.pending_store
                        .read()
                        .expect("RwLock not poisoned")
                        .get(session_id)
                        .cloned()
                        .ok_or_else(|| format!("session not found: {session_id}"))?
                }
            }
        };

        if needs_migration_from_entries(&entries) {
            entries = self.migrate_v3_to_v4(entries);
        }

        if let Some(last) = entries.last() {
            if let Some(id) = last.entry_id() {
                self.set_leaf(session_id, Some(id.to_string()));
            }
        } else {
            self.set_leaf(session_id, None);
        }

        Ok(entries)
    }

    fn parse_entries_from_content(content: &str) -> Result<Vec<SessionEntry>, String> {
        let mut entries: Vec<SessionEntry> = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: SessionEntry =
                serde_json::from_str(line).map_err(|e| format!("parse entry: {e}"))?;
            entries.push(entry);
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
    ) -> Result<Vec<SessionEntry>, String> {
        let entries = self.load(session_id).await?;
        assert_session_cwd_exists(&entries, fallback_cwd)
            .map_err(|e| format!("session validation failed: {e}"))?;
        Ok(entries)
    }

    /// Migrate v3 entries (no id/parentId) to v4.
    fn migrate_v3_to_v4(&self, entries: Vec<SessionEntry>) -> Vec<SessionEntry> {
        let mut prev_id: Option<String> = None;

        entries
            .into_iter()
            .map(|entry| match entry {
                SessionEntry::Header(h) => SessionEntry::Header(SessionHeader { version: 4, ..h }),
                _ => {
                    let new_id = Uuid::new_v4().to_string();
                    let now = Utc::now().to_rfc3339();
                    let result =
                        Self::clone_entry_with_ids(&entry, &new_id, prev_id.as_deref(), &now);
                    prev_id = Some(new_id);
                    result
                }
            })
            .collect()
    }

    /// Delete a session file and in-memory tracking (c1065 resume panel).
    pub async fn delete_session(&self, session_id: &str) -> Result<(), String> {
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
                        .map_err(|e| format!("delete session file: {e}"))?;
                }
                Ok(())
            }
        }
    }

    /// List all session IDs with metadata.
    ///
    /// Includes on-disk `.jsonl` sessions and not-yet-flushed pending sessions
    /// (created / user-only before first assistant flush).
    pub async fn list(&self) -> Result<Vec<String>, String> {
        let dir = match tokio::fs::read_dir(&self.sessions_dir).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Still surface pending-only sessions below.
                let pending = self.pending_store.read().expect("RwLock not poisoned");
                let mut pending_ids: Vec<_> = pending.keys().cloned().collect();
                pending_ids.sort();
                return Ok(pending_ids);
            }
            Err(e) => return Err(format!("read sessions dir: {e}")),
        };

        let mut entries = Vec::new();
        let mut read = dir;
        loop {
            match read.next_entry().await {
                Ok(Some(entry)) => entries.push(entry),
                Ok(None) => break,
                Err(e) => return Err(format!("read dir entry: {e}")),
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
    ) -> Result<Option<SessionEntry>, String> {
        let entries = self.load(session_id).await?;
        Ok(entries.into_iter().find(|e| e.entry_id() == Some(entry_id)))
    }

    /// Get the current leaf entry.
    pub async fn get_leaf_entry(&self, session_id: &str) -> Result<Option<SessionEntry>, String> {
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

    /// Branch: change the current leaf to a different entry.
    /// Future appends will be children of this entry.
    pub fn branch(&self, session_id: &str, entry_id: &str) {
        self.set_leaf(session_id, Some(entry_id.to_string()));
    }

    /// Reset leaf to root (null).
    pub fn reset_leaf(&self, session_id: &str) {
        self.set_leaf(session_id, None);
    }

    /// Collect entries on the path from leaf_id to root.
    pub async fn get_branch(
        &self,
        session_id: &str,
        leaf_id: Option<&str>,
    ) -> Result<Vec<SessionEntry>, String> {
        let entries = self.load(session_id).await?;
        let id_map: HashMap<&str, &SessionEntry> = entries
            .iter()
            .filter_map(|e| e.entry_id().map(|id| (id, e)))
            .collect();

        let effective_leaf = match leaf_id {
            Some(id) => id.to_string(),
            None => match entries.iter().rev().find_map(|e| e.entry_id()) {
                Some(id) => id.to_string(),
                None => return Ok(vec![]),
            },
        };

        // Collect path from leaf to root
        let mut path = Vec::new();
        let mut current = Some(effective_leaf.as_str());
        let mut visited = std::collections::HashSet::new();

        while let Some(id) = current {
            if !visited.insert(id) {
                break; // Cycle detection
            }
            if let Some(entry) = id_map.get(id) {
                path.push((*entry).clone());
                current = entry.parent_id();
            } else {
                break;
            }
        }

        path.reverse();
        Ok(path)
    }

    /// Build session context from the stored entries.
    /// Walks from leaf to root, reconstructing messages in chronological order.
    pub async fn build_session_context(&self, session_id: &str) -> Result<SessionContext, String> {
        let leaf_id = self.get_leaf(session_id);
        let branch = self.get_branch(session_id, leaf_id.as_deref()).await?;

        let mut messages = Vec::new();
        let mut thinking_level = String::from("medium");
        let mut model: Option<(String, String)> = None;

        for entry in &branch {
            match entry {
                SessionEntry::Message(m) => {
                    messages.push(m.message.clone());
                }
                SessionEntry::Compaction(c) => {
                    // Compaction summary as user-shaped AgentMessage JSON (no `system` role).
                    messages.push(crate::domain::session_types::fixture_message_json(
                        "user",
                        &format!("[Previous context summary]\n{}", c.summary),
                    ));
                }
                SessionEntry::BranchSummary(b) => {
                    messages.push(crate::domain::session_types::fixture_message_json(
                        "user",
                        &format!("[Branch summary]\n{}", b.summary),
                    ));
                }
                SessionEntry::ModelChange(mc) => {
                    model = Some((mc.provider.clone(), mc.model_id.clone()));
                }
                SessionEntry::ThinkingLevelChange(tc) => {
                    thinking_level = tc.thinking_level.clone();
                }
                SessionEntry::CustomMessage(cm) => {
                    // Custom messages participate in context as user messages
                    if cm.display {
                        messages.push(cm.content.clone());
                    }
                }
                SessionEntry::Header(_)
                | SessionEntry::Custom(_)
                | SessionEntry::Label(_)
                | SessionEntry::SessionInfo(_) => {
                    // Non-context entries: skip
                }
                SessionEntry::BashExecution(b) => {
                    // Bash executions enter context only when not explicitly excluded (`!` vs `!!`).
                    if b.exclude_from_context {
                        continue;
                    }
                    let msg = crate::domain::session_types::fixture_message_json(
                        "user",
                        &format!("$ {}\n{}", b.command, b.output),
                    );
                    messages.push(msg);
                }
            }
        }

        Ok(SessionContext {
            messages,
            thinking_level,
            model,
        })
    }

    /// Build session context as `Vec<AgentMessage>` (type-safe version).
    /// Walks from leaf to root, reconstructing messages in chronological order.
    /// Handles compaction summaries, bash execution, and branch summaries.
    pub async fn build_session_context_v2(
        &self,
        session_id: &str,
    ) -> Result<Vec<crate::domain::message::AgentMessage>, String> {
        let leaf_id = self.get_leaf(session_id);
        let branch = self.get_branch(session_id, leaf_id.as_deref()).await?;

        let mut messages = Vec::new();
        use crate::domain::message::{AgentMessage, EnvMessage};

        for entry in &branch {
            match entry {
                SessionEntry::Compaction(c) => {
                    messages.push(AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                        summary: c.summary.clone(),
                        tokens_before: c.tokens_before,
                        tokens_after: 0,
                        read_files: None,
                        modified_files: None,
                    }));
                }
                SessionEntry::BranchSummary(b) => {
                    messages.push(AgentMessage::Env(EnvMessage::BranchSummaryMessage {
                        summary: b.summary.clone(),
                        from_id: b.from_id.clone(),
                    }));
                }
                SessionEntry::CustomMessage(cm) => {
                    if cm.display {
                        let text = cm.content.as_str().unwrap_or("").to_string();
                        messages.push(AgentMessage::user(text));
                    }
                }
                SessionEntry::BashExecution(b) => {
                    if b.exclude_from_context {
                        continue;
                    }
                    messages.push(AgentMessage::bash(&b.command, &b.output, b.exit_code));
                }
                SessionEntry::Message(m) => {
                    // Try to parse message JSON into AgentMessage
                    if let Ok(msg) = serde_json::from_value::<AgentMessage>(m.message.clone()) {
                        messages.push(msg);
                    }
                }
                // Non-context entries: skip
                SessionEntry::Header(_)
                | SessionEntry::Custom(_)
                | SessionEntry::Label(_)
                | SessionEntry::SessionInfo(_)
                | SessionEntry::ModelChange(_)
                | SessionEntry::ThinkingLevelChange(_) => {}
            }
        }

        Ok(messages)
    }

    /// Get a range of branch entries between two entry IDs.
    /// Returns entries from `start_id` (inclusive) to `end_id` (exclusive).
    pub async fn get_branch_entries(
        &self,
        session_id: &str,
        start_id: &str,
        end_id: &str,
    ) -> Result<Vec<SessionEntry>, String> {
        let leaf_id = self.get_leaf(session_id);
        let branch = self.get_branch(session_id, leaf_id.as_deref()).await?;

        let mut in_range = false;
        let mut result = Vec::new();
        for entry in &branch {
            let Some(eid) = entry.entry_id() else {
                continue;
            };
            if eid == start_id {
                in_range = true;
            }
            if in_range {
                result.push(entry.clone());
            }
            if eid == end_id {
                break;
            }
        }

        Ok(result)
    }

    /// Create a branched (forked) session from a parent session up to a given entry.
    /// This is a cleaner alias for `fork()` that creates the child explicitly.
    pub async fn create_branched_session(
        &self,
        parent_id: &str,
        child_id: &str,
        target_entry_id: &str,
    ) -> Result<(), String> {
        self.fork(
            parent_id,
            child_id,
            target_entry_id,
            crate::domain::session_types::ForkPosition::At,
        )
        .await
    }

    /// Append a label to an entry.
    pub async fn append_label(
        &self,
        session_id: &str,
        target_id: &str,
        label: &str,
        description: Option<&str>,
    ) -> Result<(), String> {
        // Verify target exists
        let _ = self
            .get_entry(session_id, target_id)
            .await?
            .ok_or_else(|| format!("target entry not found: {target_id}"))?;

        let entry = SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: "label".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            target_id: target_id.to_string(),
            label: Some(if let Some(desc) = description {
                format!("{label}: {desc}")
            } else {
                label.to_string()
            }),
        });
        self.append(session_id, &entry).await
    }

    // ── Change tracking helpers ─────────────────────────────────

    /// Append a model change entry.
    pub async fn append_model_change(
        &self,
        session_id: &str,
        provider: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let entry = SessionEntry::ModelChange(ModelChangeEntry {
            base: EntryBase {
                entry_type: "model_change".into(),
                id: String::new(), // will be filled by append
                parent_id: None,
                timestamp: String::new(),
            },
            provider: provider.to_string(),
            model_id: model_id.to_string(),
        });
        self.append(session_id, &entry).await
    }

    /// Append a thinking level change entry.
    pub async fn append_thinking_level_change(
        &self,
        session_id: &str,
        level: &str,
    ) -> Result<(), String> {
        let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            base: EntryBase {
                entry_type: "thinking_level_change".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            thinking_level: level.to_string(),
        });
        self.append(session_id, &entry).await
    }

    /// Append a custom message entry (participates in LLM context).
    pub async fn append_custom_message(
        &self,
        session_id: &str,
        custom_type: &str,
        content: Value,
        display: bool,
        details: Option<Value>,
    ) -> Result<(), String> {
        let entry = SessionEntry::CustomMessage(CustomMessageEntry {
            base: EntryBase {
                entry_type: "custom_message".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            custom_type: custom_type.to_string(),
            content,
            display,
            details,
        });
        self.append(session_id, &entry).await
    }

    // ── Branch summary (text fallback) ──────────────────────────

    /// Generate a branch summary for cut-point entries.
    pub fn generate_branch_summary(&self, skipped_entries: &[SessionEntry]) -> String {
        use crate::domain::session_types::{
            count_tool_calls, is_user_message, message_text, tool_file_paths,
        };

        if skipped_entries.is_empty() {
            return String::new();
        }

        let total = skipped_entries.len();
        let user_count = skipped_entries
            .iter()
            .filter(|e| is_user_message(e))
            .count();
        let assistant_count = skipped_entries
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SessionEntry::Message(m)
                        if m.message.get("role").and_then(|r| r.as_str()) == Some("assistant")
                )
            })
            .count();

        let tool_calls: usize = skipped_entries
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Message(m) => Some(count_tool_calls(&m.message)),
                _ => None,
            })
            .sum();

        let mut files: Vec<String> = Vec::new();
        for entry in skipped_entries {
            if let SessionEntry::Message(m) = entry {
                for path in tool_file_paths(&m.message) {
                    if !files.contains(&path) {
                        files.push(path);
                    }
                }
            }
        }

        let last_user_msg = skipped_entries.iter().rev().find_map(|e| {
            if !is_user_message(e) {
                return None;
            }
            let SessionEntry::Message(m) = e else {
                return None;
            };
            let text = message_text(&m.message);
            if text.is_empty() {
                return None;
            }
            let truncated: String = text.chars().take(100).collect();
            Some(if text.chars().count() > 100 {
                format!("{truncated}...")
            } else {
                truncated
            })
        });

        let mut summary = format!("分支摘要:\n- 跳过 {total} 条记录\n");
        summary.push_str(&format!(
            "- 包含: {user_count} 条用户消息, {assistant_count} 条助手消息"
        ));
        if tool_calls > 0 {
            summary.push_str(&format!(", {tool_calls} 次工具调用"));
        }
        summary.push('\n');

        if let Some(ref msg) = last_user_msg {
            summary.push_str(&format!("- 最后一条用户消息: \"{msg}\"\n"));
        }

        if !files.is_empty() {
            files.sort();
            files.dedup();
            summary.push_str(&format!("- 涉及文件: {}\n", files.join(", ")));
        }

        summary
    }

    // ── Fork ────────────────────────────────────────────────────

    /// Fork a session: create a child session from a parent branch path.
    ///
    /// Aligns with pi `createBranchedSession`: content is [`Self::get_branch`],
    /// `parent_id` values are re-chained; the parent JSONL is never mutated.
    pub async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: crate::domain::session_types::ForkPosition,
    ) -> Result<(), String> {
        self.fork_inner(parent_id, child_id, at_entry_id, position)
            .await
    }

    /// Fork implementation (path-based; see [`crate::domain::session_types::ForkPosition`]).
    pub async fn fork_inner(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: crate::domain::session_types::ForkPosition,
    ) -> Result<(), String> {
        use crate::domain::session_types::{ForkPosition, is_user_message};

        let parent_entries = self.load(parent_id).await?;
        let selected = parent_entries
            .iter()
            .find(|e| e.entry_id() == Some(at_entry_id))
            .ok_or_else(|| format!("entry not found in parent session: {at_entry_id}"))?;

        let path_leaf: Option<&str> = match position {
            ForkPosition::At => Some(at_entry_id),
            ForkPosition::Before => {
                if !is_user_message(selected) {
                    return Err(
                        "ForkPosition::Before requires a user message entry (pi /fork)".into(),
                    );
                }
                selected.parent_id()
            }
        };

        let path = match path_leaf {
            None => Vec::new(),
            Some(leaf) => self.get_branch(parent_id, Some(leaf)).await?,
        };

        // Re-chain parent_id along the path (keep original entry ids — pi style).
        let mut rechanneled = Vec::with_capacity(path.len());
        let mut prev_id: Option<String> = None;
        for entry in path {
            let Some(eid) = entry.entry_id() else {
                continue;
            };
            let ts = entry
                .base()
                .map(|b| b.timestamp.as_str())
                .unwrap_or("")
                .to_string();
            let rewritten = Self::clone_entry_with_ids(&entry, eid, prev_id.as_deref(), &ts);
            prev_id = Some(eid.to_string());
            rechanneled.push(rewritten);
        }

        self.create(child_id, None, Some(parent_id)).await?;
        // Persist header before body rows (append_with_id writes the file directly).
        if matches!(&self.backend, SessionBackend::Persisted { .. }) {
            self.flush_pending_to_disk(child_id).await?;
        }

        for entry in &rechanneled {
            match &self.backend {
                SessionBackend::Persisted { .. } => {
                    self.append_with_id(child_id, entry).await?;
                }
                SessionBackend::InMemory { .. } => {
                    let mut store = self.in_memory_store.write().expect("RwLock not poisoned");
                    store
                        .entry(child_id.to_string())
                        .or_default()
                        .push(entry.clone());
                    if let Some(id) = entry.entry_id() {
                        self.set_leaf(child_id, Some(id.to_string()));
                    }
                }
            }
        }

        if let Some(last) = rechanneled.last().and_then(|e| e.entry_id()) {
            self.set_leaf(child_id, Some(last.to_string()));
        } else {
            self.set_leaf(child_id, None);
        }

        Ok(())
    }

    // ── Tree operations ────────────────────────────────────────

    /// Get the active session id.
    pub fn active_session_id(&self) -> Option<String> {
        self.active_session
            .read()
            .expect("RwLock not poisoned")
            .clone()
    }

    /// Set the active session.
    pub fn set_active_session(&self, id: &str) {
        self.active_session
            .write()
            .expect("RwLock not poisoned")
            .replace(id.to_string());
    }

    /// Navigate tree: change the current leaf to a different entry.
    /// Future appends will be children of this entry.
    /// Alias for `branch()`.
    pub fn navigate_tree(&self, session_id: &str, target_id: Option<&str>) {
        match target_id {
            Some(id) => self.branch(session_id, id),
            None => self.reset_leaf(session_id),
        }
    }

    /// Switch the active session to a new file path.
    /// This loads entries from the new path and updates the active session.
    pub async fn switch_session(&self, new_session_id: &str, new_path: &str) -> Result<(), String> {
        // Verify the new path exists
        let path = std::path::Path::new(new_path);
        if !path.exists() {
            return Err(format!("session file not found: {new_path}"));
        }
        // Load entries from the new path
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("read session file: {e}"))?;

        let mut entries: Vec<SessionEntry> = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: SessionEntry =
                serde_json::from_str(line).map_err(|e| format!("parse entry: {e}"))?;
            entries.push(entry);
        }

        // Update active session
        self.set_active_session(new_session_id);

        // Update leaf tracking: last entry's id
        if let Some(last) = entries.last() {
            if let Some(id) = last.entry_id() {
                self.set_leaf(new_session_id, Some(id.to_string()));
            }
        } else {
            self.set_leaf(new_session_id, None);
        }

        Ok(())
    }

    /// Get the session as a tree structure.
    /// Builds a `Vec<SessionTreeNode>` with labels resolved from LabelEntries.
    pub async fn get_tree(&self, session_id: &str) -> Result<Vec<SessionTreeNode>, String> {
        let entries = self.load(session_id).await?;
        Ok(crate::domain::session_types::build_session_tree(&entries))
    }

    // ── Label and session info ─────────────────────────────────

    /// Append a label change entry.
    /// Labels are user-defined bookmarks/markers on entries.
    pub async fn append_label_change(
        &self,
        session_id: &str,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), String> {
        // Verify target exists
        let _ = self
            .get_entry(session_id, target_id)
            .await?
            .ok_or_else(|| format!("target entry not found: {target_id}"))?;

        let entry = SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: "label".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            target_id: target_id.to_string(),
            label: label.map(String::from),
        });
        self.append(session_id, &entry).await
    }

    /// Get the label for an entry, if any.
    pub async fn get_label(
        &self,
        session_id: &str,
        target_id: &str,
    ) -> Result<Option<String>, String> {
        let entries = self.load(session_id).await?;
        // Walk in reverse to find the latest label for this target
        for entry in entries.iter().rev() {
            if let SessionEntry::Label(l) = entry
                && l.target_id == target_id
            {
                return Ok(l.label.clone());
            }
        }
        Ok(None)
    }

    /// Append a session info entry (e.g., display name).
    pub async fn append_session_info(&self, session_id: &str, name: &str) -> Result<(), String> {
        let entry = SessionEntry::SessionInfo(SessionInfoEntry {
            base: EntryBase {
                entry_type: "session_info".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            name: Some(name.trim().to_string()),
        });
        self.append(session_id, &entry).await
    }

    /// Append a bash-execution entry (`!cmd` / `!!cmd`).
    ///
    /// Stored on disk; the `exclude_from_context` flag controls whether it
    /// participates in LLM context (see `build_session_context`).
    pub async fn append_bash_execution(
        &self,
        params: BashExecutionParams<'_>,
    ) -> Result<(), String> {
        let entry = SessionEntry::BashExecution(BashExecutionEntry {
            base: EntryBase {
                entry_type: "bash_execution".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            command: params.command.to_string(),
            output: params.output.to_string(),
            exit_code: params.exit_code,
            cancelled: params.cancelled,
            truncated: params.truncated,
            full_output_path: params.full_output_path.map(|s| s.to_string()),
            exclude_from_context: params.exclude_from_context,
        });
        self.append(params.session_id, &entry).await
    }

    /// Get the current session name from the latest session_info entry.
    pub async fn get_session_name(&self, session_id: &str) -> Result<Option<String>, String> {
        let entries = self.load(session_id).await?;
        for entry in entries.iter().rev() {
            if let SessionEntry::SessionInfo(si) = entry {
                return Ok(si
                    .name
                    .clone()
                    .and_then(|n| if n.is_empty() { None } else { Some(n) }));
            }
        }
        Ok(None)
    }
}

// ── CWD Validation ──────────────────────────────────────────────────

fn needs_migration_from_entries(entries: &[SessionEntry]) -> bool {
    entries
        .iter()
        .any(|entry| matches!(entry, SessionEntry::Header(h) if h.version < 4))
}

/// Validate that the session's working directory exists.
///
/// Checks the CWD stored in the session header. If the directory does not
/// exist, tries `fallback_cwd`. Returns an error if neither is accessible.
pub fn assert_session_cwd_exists(
    entries: &[SessionEntry],
    fallback_cwd: &str,
) -> Result<(), String> {
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
        .ok_or_else(|| "session has no header entry".to_string())?;

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

    Err(format!(
        "Session working directory '{}' does not exist. Fallback '{}' also not found.",
        cwd, fallback_cwd
    ))
}

// ── XySessionStore impl ───────────────────────────────────────────────

#[async_trait::async_trait]
impl XySessionStore for SessionManager {
    async fn exists(&self, session_id: &str) -> bool {
        self.exists(session_id)
    }

    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<super::types::SessionEntry>, String> {
        SessionManager::load(self, session_id).await
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &super::types::SessionEntry,
    ) -> Result<(), String> {
        SessionManager::append(self, session_id, entry).await
    }

    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<crate::domain::session_types::SessionContext, String> {
        SessionManager::build_session_context(self, session_id).await
    }

    async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), String> {
        SessionManager::create(self, id, cwd, parent).await
    }

    async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: crate::domain::session_types::ForkPosition,
    ) -> Result<(), String> {
        SessionManager::fork(self, parent_id, child_id, at_entry_id, position).await
    }

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>) {
        <SessionManager>::set_leaf(self, session_id, entry_id.map(str::to_string));
    }

    fn leaf_id(&self, session_id: &str) -> Option<String> {
        SessionManager::get_leaf_id(self, session_id)
    }

    async fn list_sessions(
        &self,
    ) -> Result<Vec<crate::runtime_protocol::SessionListEntry>, String> {
        use crate::domain::session_types::{SessionEntry, is_user_message, message_text};
        use std::time::{SystemTime, UNIX_EPOCH};

        let ids = SessionManager::list(self).await?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let path = self.session_path(&id);
            let path_str = if path.exists() {
                Some(path.to_string_lossy().into_owned())
            } else {
                None
            };
            let name = SessionManager::get_session_name(self, &id).await?;
            let entries = SessionManager::load(self, &id).await.unwrap_or_default();
            let mut message_count = 0usize;
            let mut first_message = None;
            let mut parent_session_id = None;
            let mut modified_unix = None;
            let mut cwd = None;
            for entry in &entries {
                if let SessionEntry::Header(h) = entry {
                    parent_session_id = h.parent_session.clone();
                    if !h.cwd.is_empty() {
                        cwd = Some(h.cwd.clone());
                    }
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&h.timestamp) {
                        modified_unix = Some(dt.timestamp().max(0) as u64);
                    }
                    continue;
                }
                if matches!(entry, SessionEntry::Message(_)) {
                    message_count += 1;
                }
                if first_message.is_none()
                    && is_user_message(entry)
                    && let SessionEntry::Message(m) = entry
                {
                    let text = message_text(&m.message);
                    let cleaned = text
                        .chars()
                        .map(|c| if c.is_control() { ' ' } else { c })
                        .collect::<String>()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !cleaned.is_empty() {
                        first_message = Some(cleaned);
                    }
                }
            }
            if modified_unix.is_none() {
                let path = self.sessions_dir.join(format!("{id}.jsonl"));
                if let Ok(meta) = tokio::fs::metadata(&path).await
                    && let Ok(modified) = meta.modified()
                    && let Ok(dur) = modified.duration_since(UNIX_EPOCH)
                {
                    modified_unix = Some(dur.as_secs());
                } else {
                    modified_unix = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs());
                }
            }
            out.push(crate::runtime_protocol::SessionListEntry {
                id,
                name,
                first_message,
                message_count,
                modified_unix,
                parent_session_id,
                tree_prefix: String::new(),
                cwd,
                path: path_str,
            });
        }
        out.sort_by(|a, b| {
            b.modified_unix
                .unwrap_or(0)
                .cmp(&a.modified_unix.unwrap_or(0))
        });
        Ok(out)
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        SessionManager::delete_session(self, session_id).await
    }
}

#[cfg(test)]
mod deferred_persist_tests {
    use super::*;
    use crate::domain::session_types::{EntryBase, MessageEntry};

    fn user_message(text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            message: crate::domain::session_types::fixture_message_json("user", text),
        })
    }

    fn assistant_message(text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            message: crate::domain::session_types::fixture_message_json("assistant", text),
        })
    }

    #[tokio::test]
    async fn defer_before_assistant_keeps_disk_clean_then_flushes() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        let sid = "defer-session";

        mgr.create(sid, Some("."), None).await.unwrap();
        assert!(!mgr.session_path(sid).exists());

        mgr.append(sid, &user_message("hello")).await.unwrap();
        assert!(!mgr.session_path(sid).exists());

        let pending = mgr.load(sid).await.unwrap();
        assert!(
            pending
                .iter()
                .any(|e| matches!(e, SessionEntry::Message(_)))
        );

        mgr.append(sid, &assistant_message("hi")).await.unwrap();
        assert!(mgr.session_path(sid).exists());

        let loaded = mgr.load(sid).await.unwrap();
        let roles: Vec<_> = loaded
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Message(m) => m.message.get("role").and_then(|r| r.as_str()),
                _ => None,
            })
            .collect();
        assert!(roles.contains(&"user"));
        assert!(roles.contains(&"assistant"));
    }

    #[tokio::test]
    async fn in_memory_append_never_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::in_memory();
        let sid = "mem-session";

        mgr.create(sid, Some("."), None).await.unwrap();
        mgr.append(sid, &user_message("one")).await.unwrap();
        mgr.append(sid, &assistant_message("two")).await.unwrap();

        assert!(
            std::fs::read_dir(dir.path()).is_err() || dir.path().read_dir().unwrap().count() == 0
        );

        let loaded = mgr.load(sid).await.unwrap();
        assert_eq!(
            loaded
                .iter()
                .filter(|e| matches!(e, SessionEntry::Message(_)))
                .count(),
            2
        );
    }
}

#[cfg(test)]
mod branch_summary_tests {
    use super::*;
    use crate::domain::session_types::{EntryBase, MessageEntry, fixture_message_json};
    use serde_json::json;

    fn msg(id: &str, message: serde_json::Value) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: "t".into(),
            },
            message,
        })
    }

    #[test]
    fn branch_summary_counts_content_tool_calls_and_plain_user_text() {
        let mgr = SessionManager::in_memory();
        let entries = vec![
            msg("u1", fixture_message_json("user", "你能做什么")),
            msg(
                "a1",
                json!({
                    "role": "assistant",
                    "content": [
                        { "type": "text", "text": "查文件" },
                        { "type": "toolCall", "id": "1", "name": "read", "arguments": { "path": "src/lib.rs" } }
                    ],
                    "timestamp": 0u64,
                }),
            ),
        ];
        let summary = mgr.generate_branch_summary(&entries);
        assert!(summary.contains("1 次工具调用"), "{summary}");
        assert!(summary.contains("src/lib.rs"), "{summary}");
        assert!(summary.contains("你能做什么"), "{summary}");
        assert!(!summary.contains("{\"content\""), "{summary}");
    }
}

#[cfg(test)]
mod fork_path_tests {
    //! c645: path-based fork (pi createBranchedSession) — not file-order slice.
    use super::*;
    use crate::domain::session_types::{
        EntryBase, ForkPosition, MessageEntry, fixture_message_json, is_user_message, message_text,
    };

    fn msg(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: format!("t-{id}"),
            },
            message: fixture_message_json(role, text),
        })
    }

    /// Branched tree written so **file order** would include the sibling if
    /// sliced by index, while **get_branch** path would not:
    /// ```text
    /// u1
    /// ├─ a_left  (file earlier)
    /// └─ a_right (file later; fork target path)
    /// ```
    async fn seeded_sibling_tree(mgr: &SessionManager, sid: &str) {
        mgr.create(sid, Some("."), None).await.unwrap();
        if matches!(&mgr.backend, SessionBackend::Persisted { .. }) {
            mgr.flush_pending_to_disk(sid).await.unwrap();
        }
        for e in [
            msg("u1", None, "user", "root user"),
            msg(
                "a_left",
                Some("u1"),
                "assistant",
                "LEFT sibling — must not leak",
            ),
            msg("a_right", Some("u1"), "assistant", "RIGHT keep"),
            msg("u_right", Some("a_right"), "user", "fork me"),
        ] {
            match &mgr.backend {
                SessionBackend::Persisted { .. } => {
                    mgr.append_with_id(sid, &e).await.unwrap();
                }
                SessionBackend::InMemory { .. } => {
                    let mut store = mgr.in_memory_store.write().expect("lock");
                    store.entry(sid.to_string()).or_default().push(e.clone());
                    if let Some(id) = e.entry_id() {
                        mgr.set_leaf(sid, Some(id.to_string()));
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn fork_at_excludes_sibling_branch() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        seeded_sibling_tree(&mgr, "parent").await;

        mgr.fork("parent", "child", "u_right", ForkPosition::At)
            .await
            .unwrap();

        let child = mgr.load("child").await.unwrap();
        let ids: Vec<_> = child.iter().filter_map(|e| e.entry_id()).collect();
        assert!(
            ids.contains(&"u1") && ids.contains(&"a_right") && ids.contains(&"u_right"),
            "path must include u1→a_right→u_right: {ids:?}"
        );
        assert!(
            !ids.contains(&"a_left"),
            "sibling a_left must NOT leak into child (file-order bug): {ids:?}"
        );

        let parent = mgr.load("parent").await.unwrap();
        assert_eq!(
            parent.iter().filter(|e| e.entry_id().is_some()).count(),
            4,
            "parent must be unchanged"
        );
        let header = child.iter().find_map(|e| match e {
            SessionEntry::Header(h) => Some(h),
            _ => None,
        });
        assert_eq!(
            header.and_then(|h| h.parent_session.as_deref()),
            Some("parent")
        );
    }

    #[tokio::test]
    async fn fork_before_user_omits_user_and_prefills_source() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        seeded_sibling_tree(&mgr, "parent").await;

        mgr.fork("parent", "child", "u_right", ForkPosition::Before)
            .await
            .unwrap();

        let child = mgr.load("child").await.unwrap();
        let ids: Vec<_> = child.iter().filter_map(|e| e.entry_id()).collect();
        assert!(
            ids.contains(&"u1") && ids.contains(&"a_right"),
            "before-user path ends at parent of u_right: {ids:?}"
        );
        assert!(
            !ids.contains(&"u_right"),
            "selected user must not be copied (pi before): {ids:?}"
        );
        assert!(!ids.contains(&"a_left"), "no sibling leak: {ids:?}");

        let parent = mgr.load("parent").await.unwrap();
        let u = parent
            .iter()
            .find(|e| e.entry_id() == Some("u_right"))
            .expect("u_right");
        assert!(is_user_message(u));
        let text = match u {
            SessionEntry::Message(m) => message_text(&m.message),
            _ => String::new(),
        };
        assert_eq!(text, "fork me");
    }

    #[tokio::test]
    async fn fork_before_rejects_non_user() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        seeded_sibling_tree(&mgr, "parent").await;
        let err = mgr
            .fork("parent", "child", "a_right", ForkPosition::Before)
            .await
            .unwrap_err();
        assert!(err.contains("user"), "Before on assistant must err: {err}");
    }

    #[tokio::test]
    async fn persisted_fork_does_not_mutate_parent_file() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        seeded_sibling_tree(&mgr, "parent").await;
        // Force flush parent to disk (assistant already flushed via append_with_id).
        let before = tokio::fs::read(mgr.session_path("parent")).await.unwrap();

        mgr.fork("parent", "child", "u_right", ForkPosition::At)
            .await
            .unwrap();

        let after = tokio::fs::read(mgr.session_path("parent")).await.unwrap();
        assert_eq!(before, after, "parent JSONL bytes must be identical");
        assert!(mgr.session_path("child").exists());
    }
}

// ── XyEventSink impl ───────────────────────────────────────────────────

#[async_trait::async_trait]
impl XyEventSink for crate::infra::event::EventBus {
    async fn emit(&self, event: &XyEvent) {
        self.emit_lifecycle(event);
    }
}
