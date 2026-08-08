//! Session identity, fork, and stats on [`AgentCapabilities`].
//!
//! Product builtins / bang / export are **not** here — see `app/core` + `XyDriver`.

use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use crate::protocol::error::XyError;
use crate::protocol::message::AgentMessage;
use crate::protocol::ports::XySessionStore;

use super::{AgentCapabilities, SessionStats, observe_hook};

impl AgentCapabilities {
    // ── Session management ────────────────────────────────────────

    /// Set the active session ID.
    pub fn set_session(&mut self, session_id: String) {
        xylitol_ai_bridge::provider::set_obs_session(session_id.clone(), None);
        self.session_id = Some(session_id);
    }

    /// Get the active session ID.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Ensure a session exists with a header (create / repair if needed).
    ///
    /// `create` is idempotent: bind-then-`/model` may have already written body
    /// rows into pending; a missing header is inserted without wiping them.
    pub async fn ensure_session(&self, id: &str, parent: Option<&str>) -> Result<(), XyError> {
        let had_header = match self.store.load_entries(id).await {
            Ok(entries) => entries
                .iter()
                .any(|e| matches!(e, crate::protocol::session::SessionEntry::Header(_))),
            Err(_) => false,
        };
        let cwd_clone = self.cwd.clone();
        self.store
            .create(id, Some(&cwd_clone), parent)
            .await
            .map_err(|e| XyError::Session(anyhow::anyhow!(e)))?;
        if !had_header && let Some(bus) = &self.hook_bus {
            let reason = if parent.is_some() { "fork" } else { "new" };
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_start(reason);
            observe_hook(bus, ty, phase, ctx).await;
        }
        Ok(())
    }

    /// Load conversation messages from the session store (leaf + compaction-aware cut).
    pub(crate) async fn load_conversation_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentMessage>, XyError> {
        let entries = self
            .store
            .load_leaf_branch(session_id)
            .await
            .map_err(|e| XyError::Session(anyhow::anyhow!(e)))?;
        let entries = crate::protocol::session::build_context_entries(&entries);
        Ok(entries
            .iter()
            .filter_map(|e| e.as_agent_message())
            .collect())
    }

    /// Shared session store handle (same instance as XyDriver uses).
    pub fn session_store(&self) -> Arc<dyn XySessionStore> {
        self.store.clone()
    }

    /// Lifecycle event sink (compaction Start/End etc.).
    pub(crate) fn event_sink(&self) -> Arc<dyn crate::protocol::ports::XyEventSink> {
        self.sink.clone()
    }

    /// Compaction settings snapshot for ReAct turn-end auto.
    pub(crate) fn compaction_settings(&self) -> CompactionSettings {
        self.compaction_orchestrator.settings().clone()
    }

    // ── Fork ────────────────────────────────────────────────────

    /// Fork the current session at a given entry, creating a child session.
    ///
    /// Returns the child session ID on success. See
    /// [`crate::protocol::session::ForkPosition`].
    pub async fn fork_session(
        &self,
        at_entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyError> {
        let parent_id = self
            .session_id()
            .ok_or_else(|| XyError::Session(anyhow::anyhow!("no active session")))?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_fork(
                at_entry_id,
                &format!("{position:?}"),
            );
            observe_hook(bus, ty, phase, ctx).await;
        }

        let child_id = uuid::Uuid::new_v4().to_string();

        self.store
            .fork(parent_id, &child_id, at_entry_id, position)
            .await
            .map_err(|e| XyError::Session(anyhow::anyhow!("fork failed: {e}")))?;

        Ok(child_id)
    }

    // ── Session stats ────────────────────────────────────────────

    /// Get session statistics.
    pub async fn get_session_stats(&self) -> Result<SessionStats, XyError> {
        let sid = self
            .session_id()
            .ok_or_else(|| XyError::Session(anyhow::anyhow!("no active session")))?;
        crate::agent::capabilities::stats::compute(self.store.as_ref(), sid).await
    }
}
