//! SessionIO — session persistence via SessionStore port (HC-2).
//!
//! Thin wrapper around [`Arc<dyn SessionStore>`] isolating session-persistence
//! operations for the ReAct loop. Session management operations (create, fork,
//! navigate, export, etc.) use the concrete [`SessionManager`] directly.

use std::sync::Arc;

use crate::core::message::AgentMessage;
use crate::core::ports::SessionStore;

/// Session persistence via the SessionStore port (HC-2).
///
/// Provides only the operations the ReAct loop needs:
/// [`load_context`](Self::load_context),
/// [`append_entry`](Self::append_entry), and [`exists`](Self::exists).
/// Full session management (create, fork, navigate, export) uses the
/// concrete `SessionManager` directly through AgentSession.
#[derive(Clone)]
pub struct SessionIO {
    store: Arc<dyn SessionStore>,
}

impl SessionIO {
    pub fn new(store: Arc<dyn SessionStore>) -> Self {
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
