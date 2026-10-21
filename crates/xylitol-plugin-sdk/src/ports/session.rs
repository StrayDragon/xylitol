use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{XySessionError, XySessionStoreError};
use crate::session::{
    EntryBase, SessionContext, SessionEntry, SessionInfoEntry, SessionTreeNode, build_session_tree,
};

pub use crate::session::ForkPosition;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionListEntry {
    pub id: String,
    pub name: Option<String>,
    pub first_message: Option<String>,
    pub message_count: usize,
    pub modified_unix: Option<u64>,
    pub parent_session_id: Option<String>,
    pub tree_prefix: String,
    pub cwd: Option<String>,
    pub path: Option<String>,
}

#[async_trait]
pub trait XySessionStore: Send + Sync {
    async fn exists(&self, session_id: &str) -> bool;

    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError>;

    async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.load_entries(session_id).await
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError>;

    async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.append_session_entry(session_id, entry).await
    }

    async fn load_done_bash_ids(
        &self,
        session_id: &str,
    ) -> Result<std::collections::HashSet<String>, XySessionStoreError> {
        let entries = self.load_entries(session_id).await?;
        Ok(crate::session::done_bash_ids(&entries))
    }

    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<SessionContext, XySessionStoreError>;

    async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), XySessionStoreError>;
    async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: ForkPosition,
    ) -> Result<(), XySessionError>;

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>);

    fn leaf_id(&self, session_id: &str) -> Option<String>;

    async fn message_history_tree(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionTreeNode>, XySessionStoreError> {
        let entries = self.load_entries(session_id).await?;
        Ok(build_session_tree(&entries))
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XySessionStoreError> {
        let _ = self;
        Ok(Vec::new())
    }

    async fn get_session_name(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, XySessionStoreError> {
        let entries = self.load_entries(session_id).await?;
        for entry in entries.iter().rev() {
            if let SessionEntry::SessionInfo(si) = entry {
                return Ok(si
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string));
            }
        }
        Ok(None)
    }

    async fn set_session_name(
        &self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XySessionStoreError> {
        let sanitized = sanitize_session_display_name(name);
        let entry = SessionEntry::SessionInfo(SessionInfoEntry {
            base: EntryBase {
                entry_type: "session_info".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            name: Some(sanitized.clone()),
        });
        self.append_session_entry(session_id, &entry).await?;
        Ok(sanitized)
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        let _ = session_id;
        let _ = self;
        Err(XySessionStoreError::unsupported("delete_session"))
    }

    fn try_acquire_write_lock(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        let _ = session_id;
        Ok(())
    }

    fn release_write_lock(&self, session_id: &str) {
        let _ = session_id;
    }

    async fn read_recent(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let all = self.load_entries(session_id).await?;
        let skip = all.len().saturating_sub(limit);
        Ok(all.into_iter().skip(skip).collect())
    }
}

pub fn sanitize_session_display_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut saw_nl = false;
    for c in name.chars() {
        if c == '\r' || c == '\n' {
            if !saw_nl {
                out.push(' ');
                saw_nl = true;
            }
        } else {
            saw_nl = false;
            out.push(c);
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::session::MessageEntry;

    struct MemStore {
        entries: Mutex<Vec<SessionEntry>>,
    }

    #[async_trait]
    impl XySessionStore for MemStore {
        async fn exists(&self, _id: &str) -> bool {
            false
        }
        async fn load_entries(&self, _id: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(self.entries.lock().unwrap().clone())
        }
        async fn append_session_entry(
            &self,
            _id: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            self.entries.lock().unwrap().push(entry.clone());
            Ok(())
        }
        async fn build_session_context(
            &self,
            _id: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            Err(XySessionStoreError::unsupported("build_session_context"))
        }
        async fn create(
            &self,
            _id: &str,
            _cwd: Option<&str>,
            _parent: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            Ok(())
        }
        async fn fork(
            &self,
            _parent_id: &str,
            _child_id: &str,
            _at_entry_id: &str,
            _position: ForkPosition,
        ) -> Result<(), XySessionError> {
            Err(XySessionError::EntryNotFound {
                entry_id: _at_entry_id.into(),
            })
        }
        fn set_leaf(&self, _session_id: &str, _entry_id: Option<&str>) {}
        fn leaf_id(&self, _session_id: &str) -> Option<String> {
            None
        }
    }

    fn text_entry(id: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 1,
            },
            message: serde_json::to_value(crate::message::AgentMessage::assistant(
                text.to_string(),
            ))
            .unwrap(),
        })
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_recent_returns_last_n_in_append_order() {
        let store = MemStore {
            entries: Mutex::new(Vec::new()),
        };
        for i in 0..5 {
            store
                .append_session_entry("s", &text_entry(&format!("m{i}"), &format!("m{i}")))
                .await
                .unwrap();
        }
        let recent = store.read_recent("s", 2).await.unwrap();
        assert_eq!(recent.len(), 2, "read_recent MUST 只返回最近 limit 条");
        let last = recent.last().unwrap();
        let text = last.as_agent_message().map(|m| m.text());
        assert_eq!(text.as_deref(), Some("m4"), "返回条目 MUST 为最近");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_recent_over_limit_returns_all() {
        let store = MemStore {
            entries: Mutex::new(Vec::new()),
        };
        for i in 0..3 {
            store
                .append_session_entry("s", &text_entry(&format!("m{i}"), &format!("m{i}")))
                .await
                .unwrap();
        }
        let all = store.read_recent("s", 99).await.unwrap();
        assert_eq!(all.len(), 3, "limit 超总量 MUST 返回全部");
    }
}
