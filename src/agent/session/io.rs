//! SessionIO — session persistence, forking, navigation (spec c255 / as32).
//!
//! Thin wrapper around [`SessionManager`] isolating session-persistence
//! operations. The stub `stats()` was removed — statistics are computed in
//! [`stats`](super::stats) from the session context.

use crate::infra::session::manager::SessionManager;

/// Session persistence and navigation operations.
#[derive(Clone)]
pub struct SessionIO {
    manager: SessionManager,
}

impl SessionIO {
    pub fn new(manager: SessionManager) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> &SessionManager {
        &self.manager
    }

    /// Create a new session.
    pub async fn create(&self, id: &str, cwd: &str, parent: Option<&str>) -> Result<(), String> {
        self.manager.create(id, Some(cwd), parent).await
    }

    /// Resume an existing session (load + validate CWD).
    pub async fn load_validated(
        &self,
        id: &str,
        cwd: &str,
    ) -> Result<Vec<crate::infra::session::SessionEntry>, String> {
        self.manager.load_validated(id, cwd).await
    }

    /// Fork a session at a given entry.
    pub async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
    ) -> Result<(), String> {
        self.manager
            .fork(parent_id, child_id, at_entry_id)
            .await
            .map_err(|e| format!("fork failed: {e}"))
    }

    /// Navigate the session tree.
    pub fn navigate(&self, session_id: &str, target_id: &str) {
        self.manager.navigate_tree(session_id, Some(target_id));
    }

    /// Switch to a different session file.
    pub async fn switch(&self, new_id: &str, new_path: &str) -> Result<(), String> {
        self.manager.switch_session(new_id, new_path).await
    }

    /// Persist a model change entry.
    pub async fn append_model_change(
        &self,
        session_id: &str,
        provider: &str,
        model_id: &str,
    ) -> Result<(), String> {
        self.manager
            .append_model_change(session_id, provider, model_id)
            .await
    }

    /// Persist a thinking level change entry.
    pub async fn append_thinking_level_change(
        &self,
        session_id: &str,
        level: &str,
    ) -> Result<(), String> {
        self.manager
            .append_thinking_level_change(session_id, level)
            .await
    }
}
