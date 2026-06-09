//! SessionManager — JSONL file-based session storage.
//!
//! Handles create, append, load, list, exists for sessions.

use std::path::PathBuf;

use chrono::Utc;
use uuid::Uuid;

use super::types::*;

/// Manages session persistence using JSONL files.
#[derive(Clone, Debug, Default)]
pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    /// Create a new SessionManager with the given sessions directory.
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    /// Default sessions directory: ~/.xylitol/sessions/
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol")
            .join("sessions")
    }

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
            "version": SESSION_VERSION,
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

        Ok(())
    }

    /// Append an entry to a session's JSONL file.
    pub async fn append(&self, session_id: &str, entry: &SessionEntry) -> Result<(), String> {
        let path = self.session_path(session_id);
        let line = serde_json::to_string(entry).map_err(|e| format!("serialize entry: {e}"))?;
        let content = format!("{line}\n");

        // Append with file locking (best-effort via atomic write on Unix)
        tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await
            .map_err(|e| format!("open session file: {e}"))?;

        // Use tokio::fs::write with append — actually let's just append properly
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .await
            .map_err(|e| format!("open for append: {e}"))?;
        file.write_all(content.as_bytes())
            .await
            .map_err(|e| format!("write entry: {e}"))?;

        Ok(())
    }

    /// Load all entries from a session file.
    pub async fn load(&self, session_id: &str) -> Result<Vec<SessionEntry>, String> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return Err(format!("session not found: {session_id}"));
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("read session: {e}"))?;

        let mut entries = Vec::new();
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

    /// List all session IDs.
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

        // Sort by modification time, most recent first
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

    /// Generate a branch summary for cut-point entries.
    pub fn generate_branch_summary(&self, skipped_entries: &[SessionEntry]) -> String {
        if skipped_entries.is_empty() {
            return String::new();
        }

        let total = skipped_entries.len();
        let user_count = skipped_entries
            .iter()
            .filter(|e| matches!(e, SessionEntry::Message(m) if m.message.get("role").and_then(|r| r.as_str()) == Some("user")))
            .count();
        let assistant_count = skipped_entries
            .iter()
            .filter(|e| matches!(e, SessionEntry::Message(m) if m.message.get("role").and_then(|r| r.as_str()) == Some("assistant")))
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

    /// Fork a session: create a child session from a parent up to a given entry.
    ///
    /// Copies entries [0..at_entry_index+1] from parent, then appends a
    /// branch_summary for any skipped entries.
    pub async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
    ) -> Result<(), String> {
        // Load parent entries
        let parent_entries = self.load(parent_id).await?;

        // Find the fork point
        let fork_index = parent_entries
            .iter()
            .position(|e| e.entry_id() == Some(at_entry_id))
            .ok_or_else(|| format!("entry not found in parent session: {at_entry_id}"))?;

        // Split: kept = [0..fork_index+1], skipped = [fork_index+1..]
        let kept = &parent_entries[..=fork_index];
        let skipped = if fork_index + 1 < parent_entries.len() {
            &parent_entries[fork_index + 1..]
        } else {
            &[]
        };

        // Create child session with parent link
        self.create(child_id, None, Some(parent_id)).await?;

        // Write kept entries to child session
        for entry in kept {
            self.append(child_id, entry).await?;
        }

        // Generate and write branch summary for skipped entries
        if !skipped.is_empty() {
            let summary = self.generate_branch_summary(skipped);
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
            self.append(child_id, &branch_entry).await?;
        }

        Ok(())
    }
}
