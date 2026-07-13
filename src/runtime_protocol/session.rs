//! Runtime boundary for session persistence.

use async_trait::async_trait;

use crate::domain::session_types::{
    SessionContext, SessionEntry, SessionTreeNode, build_session_tree,
};

/// Persistence port — abstracts session storage so the agent can be
/// unit-tested without a real filesystem and the server can host
/// sessions without coupling to the file store.
///
/// Methods are the **minimum** the ReAct loop and compaction call.
/// Full session management (fork/navigate/export) stays on the concrete
/// `infra::session::SessionManager` for the composition root.
#[async_trait]
pub trait XySessionStore: Send + Sync {
    /// Check whether a session exists.
    async fn exists(&self, session_id: &str) -> bool;

    /// Load the raw typed session entries (for compaction / export).
    async fn load_entries(&self, session_id: &str) -> Result<Vec<SessionEntry>, String>;
    /// Append a typed session entry (compaction summary, bash exec, etc.).
    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), String>;
    /// Build the full session context (messages, thinking level, model).
    async fn build_session_context(&self, session_id: &str) -> Result<SessionContext, String>;

    /// Create a new session (writes header, initializes leaf tracking).
    async fn create(&self, id: &str, cwd: Option<&str>, parent: Option<&str>)
    -> Result<(), String>;
    /// Fork a session at a given entry into a new child session.
    async fn fork(&self, parent_id: &str, child_id: &str, at_entry_id: &str) -> Result<(), String>;

    /// Set the active leaf entry for branching / travel.
    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>);

    /// Current leaf entry id, if any.
    fn leaf_id(&self, session_id: &str) -> Option<String>;

    /// Build the message-history session tree (default: load entries + [`build_session_tree`]).
    async fn message_history_tree(&self, session_id: &str) -> Result<Vec<SessionTreeNode>, String> {
        let entries = self.load_entries(session_id).await?;
        Ok(build_session_tree(&entries))
    }
}
