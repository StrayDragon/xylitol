use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::error::{XyError, XySessionError};
use xylitol_plugin_sdk::message::AgentMessage;

use super::{AgentCapabilities, SessionStats, observe_hook};

pub async fn load_conversation_history_from_store(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<Vec<AgentMessage>, XyError> {
    let entries = store
        .load_leaf_branch(session_id)
        .await
        .map_err(XyError::from)?;
    let entries = xylitol_plugin_sdk::session::build_context_entries(&entries);
    let messages: Vec<AgentMessage> = entries
        .iter()
        .filter_map(|entry| entry.as_agent_message())
        .collect();

    match store.load_done_bash_ids(session_id).await {
        Ok(done) => Ok(xylitol_plugin_sdk::session::fold_interrupted_bash_rows(
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
    pub fn set_session(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    pub(crate) fn obs_session_snapshot(&self) -> xylitol_plugin_sdk::ObsSessionContext {
        xylitol_plugin_sdk::ObsSessionContext {
            session_id: self.session_id.clone(),
            session_name: self.session_name.clone(),
            ..Default::default()
        }
    }

    pub(crate) fn set_session_name_fact(&mut self, name: Option<String>) {
        self.session_name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    }

    pub(crate) fn session_name(&self) -> Option<&str> {
        self.session_name.as_deref()
    }

    pub(crate) async fn obs_session_snapshot_from_store(
        &self,
    ) -> xylitol_plugin_sdk::ObsSessionContext {
        let mut ctx = self.obs_session_snapshot();
        let Some(sid) = ctx.session_id.clone() else {
            return ctx;
        };
        if let Ok(entries) = self.store.load_entries(&sid).await {
            let (parent, cut) = xylitol_plugin_sdk::session::session_fork_edge(&entries);
            ctx.parent_session_id = parent;
            ctx.fork_at_entry_id = cut;
        }
        ctx
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub async fn ensure_session(&self, id: &str, parent: Option<&str>) -> Result<(), XyError> {
        let had_header = match self.store.load_entries(id).await {
            Ok(entries) => entries
                .iter()
                .any(|e| matches!(e, xylitol_plugin_sdk::session::SessionEntry::Header(_))),
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

    pub async fn load_conversation_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentMessage>, XyError> {
        load_conversation_history_from_store(self.store.as_ref(), session_id).await
    }

    pub fn session_store(&self) -> Arc<dyn XySessionStore> {
        self.store.clone()
    }

    pub(crate) fn event_sink(&self) -> Arc<dyn xylitol_plugin_sdk::XyEventSink> {
        self.sink.clone()
    }

    pub(crate) fn compaction_settings(&self) -> CompactionSettings {
        self.compaction_orchestrator.settings().clone()
    }

    pub(crate) fn compaction(&self) -> Arc<dyn crate::agent::compaction::XyCompaction> {
        Arc::clone(&self.compaction_orchestrator)
    }

    pub async fn fork_session(
        &self,
        at_entry_id: &str,
        position: xylitol_plugin_sdk::session::ForkPosition,
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

    pub async fn get_session_stats(&self) -> Result<SessionStats, XyError> {
        let sid = self
            .session_id()
            .ok_or(XyError::from(XySessionError::NoActiveSession))?;
        crate::agent::capabilities::stats::compute(self.store.as_ref(), sid).await
    }
}
