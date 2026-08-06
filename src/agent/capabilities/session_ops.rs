//! Session identity, fork, bash, export, and queue APIs on [`AgentCapabilities`].

use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use crate::agent::prompt::commands::SlashCommandInfo;
use crate::protocol::error::XyError;
use crate::protocol::message::AgentMessage;
use crate::protocol::ports::{XyPermission, XySessionStore};

use super::{AgentCapabilities, SessionStats, cancel_hook, observe_hook};

impl AgentCapabilities {
    /// Skill/extension slash commands only (product builtins are app-owned).
    pub(crate) fn extension_commands(&self) -> &[SlashCommandInfo] {
        &self.extension_commands
    }

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

    /// Ensure a session exists (create if needed).
    pub async fn ensure_session(&self, id: &str, parent: Option<&str>) -> Result<(), XyError> {
        if !self.store.exists(id).await {
            let cwd_clone = self.cwd.clone();
            self.store
                .create(id, Some(&cwd_clone), parent)
                .await
                .map_err(|e| XyError::Session(anyhow::anyhow!(e)))?;
            if let Some(bus) = &self.hook_bus {
                let reason = if parent.is_some() { "fork" } else { "new" };
                let (ty, phase, ctx) =
                    crate::agent::runtime::script_hook_ctx::session_start(reason);
                observe_hook(bus, ty, phase, ctx).await;
            }
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

    /// Get session statistics.
    pub async fn get_session_stats(&self) -> Result<SessionStats, XyError> {
        let sid = self
            .session_id()
            .ok_or_else(|| XyError::Session(anyhow::anyhow!("no active session")))?;
        crate::agent::capabilities::stats::compute(self.store.as_ref(), sid).await
    }

    // ── Bash execution (`!cmd` / `!!cmd`) ───────────────────────

    /// Execute a user-initiated bash command and record the result.
    ///
    /// `exclude_from_context=true` (the `!!` prefix) stores the entry on disk
    /// but omits it from LLM context (see `build_session_context`).
    ///
    /// Takes `&self` so an in-flight bash can be cancelled via [`Self::abort_bash`]
    /// / [`crate::agent::AgentRuntime::abort`] without an exclusive borrow.
    pub async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<crate::protocol::ports::XyBashResult, XyError> {
        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::user_bash(
                command,
                exclude_from_context,
                &self.cwd,
            );
            cancel_hook(bus, ty, phase, ctx)
                .await
                .map_err(|e| XyError::Session(anyhow::anyhow!(e)))?;
        }
        let store: &dyn XySessionStore = self.store.as_ref();
        let sid = self.session_id().map(str::to_string);
        self.bash
            .execute(
                store,
                sid.as_deref(),
                command,
                exclude_from_context,
                chunk_tx,
            )
            .await
    }

    /// Persist a bash result as `SessionEntry::Message` with `role=bashExecution`.
    pub async fn record_bash_result(
        &self,
        command: &str,
        result: &crate::protocol::ports::XyBashResult,
        exclude_from_context: bool,
        session_id: Option<&str>,
    ) -> Result<(), XyError> {
        let sid = match session_id {
            Some(s) => s.to_string(),
            None => self
                .session_id()
                .ok_or_else(|| XyError::Session(anyhow::anyhow!("no active session")))?
                .to_string(),
        };
        crate::agent::capabilities::bash::record_bash_result(
            self.store.as_ref(),
            command,
            result,
            exclude_from_context,
            &sid,
        )
        .await
    }

    /// Get a reference to the permission engine (injected at construction).
    pub fn get_permission(&self) -> std::sync::Arc<dyn XyPermission> {
        self.permission.clone()
    }

    /// Abort any in-flight bash execution (`&self` so [`crate::agent::AgentRuntime::abort`] can call it).
    pub fn abort_bash(&self) {
        self.bash.abort();
    }

    // ── Lifecycle management ───────────────────────────────────────

    // ── Export / import (delegated to SessionExporter) ─────────

    /// Export the active session's entries to an HTML file. Returns the path.
    pub async fn export_to_html(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, XyError> {
        let sid = self
            .session_id()
            .ok_or_else(|| XyError::Session(anyhow::anyhow!("no active session")))?
            .to_string();
        self.exporter
            .export_to_html(self.store.as_ref(), &sid, path)
            .await
    }

    /// Export the active session's entries as JSONL. Returns the path.
    pub async fn export_to_jsonl(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, XyError> {
        let sid = self
            .session_id()
            .ok_or_else(|| XyError::Session(anyhow::anyhow!("no active session")))?
            .to_string();
        self.exporter
            .export_to_jsonl(self.store.as_ref(), &sid, path)
            .await
    }

    /// Import a JSONL file into a brand-new session. Returns the new session id.
    ///
    /// The new session id is derived from the source header (re-used) to keep
    /// identities stable across export/import; the file lands without
    /// overwriting an existing session.
    pub async fn import_from_jsonl(&self, path: &std::path::Path) -> Result<String, XyError> {
        self.exporter
            .import_from_jsonl(self.store.as_ref(), path)
            .await
    }
}
