//! SessionIO — session persistence via XySessionStore port (HC-2).
//!
//! Thin wrapper around [`Arc<dyn XySessionStore>`] isolating session-persistence
//! operations for the ReAct loop. Session management operations (create, fork,
//! navigate, export, etc.) use the concrete [`SessionManager`] directly.

use std::sync::Arc;

use crate::domain::message::AgentMessage;
use crate::runtime_protocol::XySessionStore;

/// Session persistence via the XySessionStore port (HC-2).
///
/// Provides only the operations the ReAct loop needs:
/// [`load_context`](Self::load_context),
/// [`append_entry`](Self::append_entry), and [`exists`](Self::exists).
/// Full session management (create, fork, navigate, export) uses the
/// concrete `SessionManager` directly through AgentSession.
#[derive(Clone)]
pub struct SessionIO {
    store: Arc<dyn XySessionStore>,
}

impl SessionIO {
    pub fn new(store: Arc<dyn XySessionStore>) -> Self {
        Self { store }
    }

    /// Load session context for building turn state.
    pub async fn load_context(&self, session_id: &str) -> Result<Vec<AgentMessage>, String> {
        self.store.load_context(session_id).await
    }

    /// Append an opaque JSON entry to the session log.
    pub async fn append_entry(
        &self,
        session_id: &str,
        entry: serde_json::Value,
    ) -> Result<(), String> {
        self.store.append_entry(session_id, entry).await
    }

    /// Check whether a session exists.
    pub async fn exists(&self, session_id: &str) -> bool {
        self.store.exists(session_id).await
    }
}
