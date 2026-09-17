//! Session identity, fork, and stats on [`AgentCapabilities`].
//!
//! Product builtins / bang / export are **not** here — see `app/core` + `XyDriver`.

use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use crate::protocol::error::{XyError, XySessionError};
use crate::protocol::message::AgentMessage;
use crate::protocol::ports::XySessionStore;

use super::{AgentCapabilities, SessionStats, observe_hook};

pub(crate) async fn load_conversation_history_from_store(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<Vec<AgentMessage>, XyError> {
    let entries = store
        .load_leaf_branch(session_id)
        .await
        .map_err(XyError::from)?;
    let entries = crate::protocol::session::build_context_entries(&entries);
    let messages: Vec<AgentMessage> = entries
        .iter()
        .filter_map(|entry| entry.as_agent_message())
        .collect();

    match store.load_done_bash_ids(session_id).await {
        Ok(done) => Ok(crate::protocol::session::fold_interrupted_bash_rows(
            messages, &done,
        )),
        Err(error) => {
            log::warn!(
                target: "xylitol::session",
                "interrupted-bash fold skipped: load_done_bash_ids failed error={error}"
            );
            Ok(messages)
        }
    }
}

impl AgentCapabilities {
    // ── Session management ────────────────────────────────────────

    /// Set the active session ID.
    ///
    /// Writes the process obs slot only when `Self::obs_slot_writes` is on
    /// (otel25: host reader drivers opt out).
    pub fn set_session(&mut self, session_id: String) {
        if self.obs_slot_writes {
            xylitol_ai_bridge::provider::set_obs_session(session_id.clone(), None);
        }
        self.session_id = Some(session_id);
    }

    /// Obs-slot write permission for `set_session` (otel25). Default on;
    /// host reader drivers turn this off before binding.
    pub(crate) fn set_obs_slot_writes(&mut self, enabled: bool) {
        self.obs_slot_writes = enabled;
    }

    pub(crate) fn obs_slot_writes(&self) -> bool {
        self.obs_slot_writes
    }

    /// This session's obs identity snapshot (otel24 / c2610): the bound bookmark
    /// id plus the slot's display name. Compaction / idle callers with a known
    /// session MUST build the snapshot here instead of re-reading the slot id.
    pub(crate) fn obs_session_snapshot(&self) -> xylitol_ai_bridge::ObsSessionContext {
        xylitol_ai_bridge::ObsSessionContext {
            session_id: self.session_id.clone(),
            session_name: xylitol_ai_bridge::provider::obs_session_context().session_name,
            ..Default::default()
        }
    }

    /// Snapshot plus header tree edge (facts from disk). Idle callers may skip this.
    pub(crate) async fn obs_session_snapshot_from_store(
        &self,
    ) -> xylitol_ai_bridge::ObsSessionContext {
        let mut ctx = self.obs_session_snapshot();
        let Some(sid) = ctx.session_id.clone() else {
            return ctx;
        };
        if let Ok(entries) = self.store.load_entries(&sid).await {
            let (parent, cut) = crate::protocol::session::session_fork_edge(&entries);
            ctx.parent_session_id = parent;
            ctx.fork_at_entry_id = cut;
        }
        ctx
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
            .map_err(XyError::from)?;
        if !had_header && let Some(bus) = &self.hook_bus {
            let reason = if parent.is_some() { "fork" } else { "new" };
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_start(reason);
            observe_hook(bus, ty, phase, ctx).await;
        }
        Ok(())
    }

    /// Load conversation messages from the session store (leaf + compaction-aware cut).
    ///
    /// c2770 / as-bang1: orphan running bash rows fold into the pinned
    /// interrupted notice here, paired against a session-scoped done set. On a
    /// done-set read failure the fold is skipped entirely (running rows keep
    /// the no-projection behavior — no false interrupts).
    pub(crate) async fn load_conversation_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentMessage>, XyError> {
        load_conversation_history_from_store(self.store.as_ref(), session_id).await
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
            .ok_or(XyError::from(XySessionError::NoActiveSession))?;

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
            .map_err(XyError::from)?;

        Ok(child_id)
    }

    // ── Session stats ────────────────────────────────────────────

    /// Get session statistics.
    pub async fn get_session_stats(&self) -> Result<SessionStats, XyError> {
        let sid = self
            .session_id()
            .ok_or(XyError::from(XySessionError::NoActiveSession))?;
        crate::agent::capabilities::stats::compute(self.store.as_ref(), sid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::SessionManager;
    use crate::protocol::ports::XySessionStore;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};

    fn message(id: &str, parent_id: Option<&str>, role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent_id.map(str::to_owned),
                timestamp: 1,
            },
            message: crate::protocol::session::fixture_message_json(role, text),
        })
    }

    #[tokio::test]
    async fn history_read_does_not_move_travelled_leaf() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = SessionManager::new(dir.path().join("sessions"));
        let sid = "travel-history";
        mgr.create(sid, Some("."), None).await.unwrap();
        for entry in [
            message("u1", None, "user", "root"),
            message("a1", Some("u1"), "assistant", "root answer"),
            message("u2", Some("a1"), "user", "latest"),
            message("a2", Some("u2"), "assistant", "latest answer"),
        ] {
            mgr.append_with_id(sid, &entry).await.unwrap();
        }

        <SessionManager as XySessionStore>::set_leaf(&mgr, sid, Some("u1"));
        load_conversation_history_from_store(&mgr, sid)
            .await
            .unwrap();
        assert_eq!(
            <SessionManager as XySessionStore>::leaf_id(&mgr, sid).as_deref(),
            Some("u1")
        );

        mgr.append(sid, &message("", None, "assistant", "continued"))
            .await
            .unwrap();
        let entries = mgr.load(sid).await.unwrap();
        let continued = entries
            .iter()
            .find(|entry| {
                matches!(
                    entry,
                    SessionEntry::Message(message)
                        if crate::protocol::session::message_text(&message.message)
                            == "continued"
                )
            })
            .expect("continued message");
        assert_eq!(continued.parent_id(), Some("u1"));
    }
}
