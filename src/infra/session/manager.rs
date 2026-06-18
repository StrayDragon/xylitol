//! SessionManager — JSONL file-based session storage.
//!
//! Handles create, append, load, list, exists, tree navigation,
//! build_session_context, and version migration for sessions.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::compaction;
use super::types::*;

/// Manages session persistence using JSONL files.
#[derive(Debug)]
pub struct SessionManager {
    sessions_dir: PathBuf,
    /// Per-session leaf node tracking (in-memory).
    /// session_id -> current leaf entry id (None = root).
    leaf_ids: RwLock<HashMap<String, Option<String>>>,
    /// Active session tracking.
    active_session: RwLock<Option<String>>,
}

impl Clone for SessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions_dir: self.sessions_dir.clone(),
            leaf_ids: RwLock::new(self.leaf_ids.read().expect("RwLock not poisoned").clone()),
            active_session: RwLock::new(
                self.active_session
                    .read()
                    .expect("RwLock not poisoned")
                    .clone(),
            ),
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            sessions_dir: PathBuf::from("."),
            leaf_ids: RwLock::new(HashMap::new()),
            active_session: RwLock::new(None),
        }
    }
}

impl SessionManager {
    /// Create a new SessionManager with the given sessions directory.
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            sessions_dir,
            leaf_ids: RwLock::new(HashMap::new()),
            active_session: RwLock::new(None),
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

    // ── CRUD ────────────────────────────────────────────────────

    /// Get the file path for a session.
    fn session_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{id}.jsonl"))
    }

    /// Check if a session exists.
    pub fn exists(&self, id: &str) -> bool {
        self.session_path(id).exists()
    }

    /// Create a new session and write the header entry.
    pub async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent_session: Option<&str>,
    ) -> Result<(), String> {
        let path = self.session_path(id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create sessions dir: {e}"))?;
        }

        // Build header JSON manually (not via enum tag to avoid duplicate `type`).
        let mut header = serde_json::json!({
            "type": "session",
            "version": 4, // v4: id/parentId tree structure
            "id": id,
            "timestamp": Utc::now().to_rfc3339(),
            "cwd": cwd.unwrap_or(".")
        });
        if let Some(ps) = parent_session {
            header["parent_session"] = serde_json::json!(ps);
        }

        let line = serde_json::to_string(&header).map_err(|e| format!("serialize header: {e}"))?;
        tokio::fs::write(&path, format!("{line}\n"))
            .await
            .map_err(|e| format!("write session file: {e}"))?;

        // Initialize leaf tracking (root = null)
        self.set_leaf(id, None);

        Ok(())
    }

    /// Append an entry to a session's JSONL file.
    /// Automatically generates id and links parent_id from current leaf.
    pub async fn append(&self, session_id: &str, entry: &SessionEntry) -> Result<(), String> {
        let path = self.session_path(session_id);

        // Build entry with auto-generated id and parent link
        let entry_with_ids = self.inject_ids(session_id, entry);

        let line =
            serde_json::to_string(&entry_with_ids).map_err(|e| format!("serialize entry: {e}"))?;
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

        // Update leaf pointer
        if let Some(new_id) = entry_with_ids.entry_id() {
            self.set_leaf(session_id, Some(new_id.to_string()));
        }

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
        }
    }

    /// Append an entry with explicit id (for fork operations).
    pub async fn append_with_id(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), String> {
        let path = self.session_path(session_id);
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

    /// Load all entries from a session file (with v3→v4 migration if needed).
    pub async fn load(&self, session_id: &str) -> Result<Vec<SessionEntry>, String> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Err(format!("session not found: {session_id}"));
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("read session: {e}"))?;

        let mut entries: Vec<SessionEntry> = Vec::new();
        let mut needs_migration = false;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: SessionEntry =
                serde_json::from_str(line).map_err(|e| format!("parse entry: {e}"))?;

            // Check header version
            if let SessionEntry::Header(ref h) = entry
                && h.version < 4
            {
                needs_migration = true;
            }

            entries.push(entry);
        }

        if needs_migration {
            entries = self.migrate_v3_to_v4(entries);
        }

        // Update leaf tracking: last entry's id
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

    /// List all session IDs with metadata.
    pub async fn list(&self) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        let dir = match tokio::fs::read_dir(&self.sessions_dir).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ids),
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
        ids = files.into_iter().map(|(id, _)| id).collect();

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
                    // Compaction summary as system message
                    let summary_msg = serde_json::json!({
                        "role": "system",
                        "parts": [{"type": "text", "text": format!("[Previous context summary]\n{}", c.summary)}]
                    });
                    messages.push(summary_msg);
                }
                SessionEntry::BranchSummary(b) => {
                    let summary_msg = serde_json::json!({
                        "role": "system",
                        "parts": [{"type": "text", "text": format!("[Branch summary]\n{}", b.summary)}]
                    });
                    messages.push(summary_msg);
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
            }
        }

        Ok(SessionContext {
            messages,
            thinking_level,
            model,
        })
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
        if skipped_entries.is_empty() {
            return String::new();
        }

        let total = skipped_entries.len();
        let user_count = skipped_entries
            .iter()
            .filter(|e| {
                matches!(e, SessionEntry::Message(m) if m.message.get("role").and_then(|r| r.as_str()) == Some("user"))
            })
            .count();
        let assistant_count = skipped_entries
            .iter()
            .filter(|e| {
                matches!(e, SessionEntry::Message(m) if m.message.get("role").and_then(|r| r.as_str()) == Some("assistant"))
            })
            .count();

        // Count tool calls from assistant messages
        let tool_calls: usize = skipped_entries
            .iter()
            .filter_map(|e| {
                if let SessionEntry::Message(m) = e {
                    m.message
                        .get("parts")
                        .and_then(|p| p.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter(|p| {
                                    p.get("type").and_then(|t| t.as_str()) == Some("FunctionCall")
                                })
                                .count()
                        })
                } else {
                    None
                }
            })
            .sum();

        // Extract files from tool calls
        let mut files: Vec<String> = Vec::new();
        for entry in skipped_entries {
            if let SessionEntry::Message(m) = entry
                && let Some(parts) = m.message.get("parts").and_then(|p| p.as_array())
            {
                for part in parts {
                    if part.get("type").and_then(|t| t.as_str()) == Some("FunctionCall") {
                        let name = part.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        if matches!(name, "read" | "write" | "edit")
                            && let Some(path) = part
                                .get("args")
                                .and_then(|a| a.get("path"))
                                .and_then(|p| p.as_str())
                            && !files.contains(&path.to_string())
                        {
                            files.push(path.to_string());
                        }
                    }
                }
            }
        }

        // Find last user message
        let last_user_msg = skipped_entries.iter().rev().find_map(|e| {
            if let SessionEntry::Message(m) = e {
                if m.message.get("role").and_then(|r| r.as_str()) == Some("user") {
                    m.message
                        .get("parts")
                        .and_then(|p| p.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|p| p.get("text"))
                        .and_then(|t| t.as_str())
                        .map(|s| {
                            let truncated: String = s.chars().take(100).collect();
                            if s.len() > 100 {
                                format!("{truncated}...")
                            } else {
                                truncated
                            }
                        })
                } else {
                    None
                }
            } else {
                None
            }
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

    /// Fork a session: create a child session from a parent up to a given entry.
    pub async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
    ) -> Result<(), String> {
        self.fork_with_model(parent_id, child_id, at_entry_id, None)
            .await
    }

    /// Fork with optional LLM model for branch summary.
    pub async fn fork_with_model(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        model: Option<&dyn crate::agent::traits::XyModel>,
    ) -> Result<(), String> {
        let parent_entries = self.load(parent_id).await?;

        let fork_index = parent_entries
            .iter()
            .position(|e| e.entry_id() == Some(at_entry_id))
            .ok_or_else(|| format!("entry not found in parent session: {at_entry_id}"))?;

        let kept = &parent_entries[..=fork_index];
        let skipped = if fork_index + 1 < parent_entries.len() {
            &parent_entries[fork_index + 1..]
        } else {
            &[]
        };

        self.create(child_id, None, Some(parent_id)).await?;

        for entry in kept {
            self.append_with_id(child_id, entry).await?;
        }

        if !skipped.is_empty() {
            let summary = if let Some(m) = model {
                match compaction::generate_branch_summary_llm(skipped, m, 16384).await {
                    Some(r) => r.summary,
                    None => self.generate_branch_summary(skipped), // fallback
                }
            } else {
                self.generate_branch_summary(skipped)
            };
            let now = Utc::now().to_rfc3339();
            let branch_entry = SessionEntry::BranchSummary(BranchSummaryEntry {
                base: EntryBase {
                    entry_type: "branch_summary".into(),
                    id: Uuid::new_v4().to_string(),
                    parent_id: Some(at_entry_id.to_string()),
                    timestamp: now,
                },
                from_id: at_entry_id.to_string(),
                summary,
                details: None,
                from_hook: Some(false),
            });
            self.append_with_id(child_id, &branch_entry).await?;
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
        Ok(Self::build_tree(&entries))
    }

    /// Build a tree from entries (public for testing).
    pub fn build_tree(entries: &[SessionEntry]) -> Vec<SessionTreeNode> {
        use std::collections::HashMap;

        // Collect labels from LabelEntries
        let mut labels: HashMap<String, String> = HashMap::new();
        for entry in entries {
            if let SessionEntry::Label(l) = entry {
                if let Some(ref label) = l.label {
                    labels.insert(l.target_id.clone(), label.clone());
                } else {
                    labels.remove(&l.target_id);
                }
            }
        }

        let mut node_map: HashMap<String, SessionTreeNode> = HashMap::new();
        let mut roots: Vec<SessionTreeNode> = Vec::new();

        // Create nodes
        for entry in entries {
            if entry.entry_type() == "label" || entry.entry_type() == "session" {
                continue; // Labels and headers not part of tree display
            }
            if let Some(id) = entry.entry_id() {
                let label = labels.get(id).cloned();
                node_map.insert(
                    id.to_string(),
                    SessionTreeNode {
                        entry: entry.clone(),
                        children: Vec::new(),
                        label,
                    },
                );
            }
        }

        // Build tree connections
        for entry in entries {
            if entry.entry_type() == "label" || entry.entry_type() == "session" {
                continue;
            }
            let Some(id) = entry.entry_id() else { continue };
            let Some(node) = node_map.remove(id) else {
                continue;
            };

            if let Some(parent_id) = entry.parent_id() {
                if let Some(parent) = node_map.get_mut(parent_id) {
                    parent.children.push(node);
                } else {
                    // Orphan - treat as root
                    roots.push(node);
                }
            } else {
                roots.push(node);
            }
        }

        // Sort children by timestamp
        fn sort_children(nodes: &mut [SessionTreeNode]) {
            for node in nodes.iter_mut() {
                node.children.sort_by(|a, b| {
                    let ta = a.entry.base().map(|b| b.timestamp.clone());
                    let tb = b.entry.base().map(|b| b.timestamp.clone());
                    ta.cmp(&tb)
                });
                sort_children(&mut node.children);
            }
        }
        sort_children(&mut roots);

        roots
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
