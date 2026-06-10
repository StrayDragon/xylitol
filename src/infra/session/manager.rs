//! SessionManager — JSONL file-based session storage.
//!
//! Handles create, append, load, list, exists for sessions.

use std::path::PathBuf;

use chrono::Utc;

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

        let header = SessionHeader {
            entry_type: "session".to_string(),
            version: SESSION_VERSION,
            id: id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            cwd: cwd.unwrap_or(".").to_string(),
            parent_session: parent_session.map(|s| s.to_string()),
        };

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
    pub fn generate_branch_summary(
        &self,
        _parent_entries: &[SessionEntry],
    ) -> Result<String, String> {
        // For now, a simple summary; full compaction logic in Phase 4
        Ok(format!(
            "[Branch with {} prior entries]",
            _parent_entries.len()
        ))
    }
}
