//! Runtime boundary for session persistence.

use async_trait::async_trait;

use crate::domain::message::AgentMessage;
use crate::domain::session_types::{SessionContext, SessionEntry};

/// Persistence port — abstracts session storage so the agent can be
/// unit-tested without a real filesystem and the server can host
/// sessions without coupling to the file store.
///
/// Methods are the **minimum** the ReAct loop and compaction call.
/// Full session management (fork/navigate/export) stays on the concrete
/// `infra::session::SessionManager` for the composition root.
#[async_trait]
pub trait XySessionStore: Send + Sync {
    /// Load session context (messages, model, CWD) for building turn state.
    async fn load_context(&self, session_id: &str) -> Result<Vec<AgentMessage>, String>;
    /// Append an opaque JSON entry to the session log.
    async fn append_entry(&self, session_id: &str, entry: serde_json::Value) -> Result<(), String>;
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
}
