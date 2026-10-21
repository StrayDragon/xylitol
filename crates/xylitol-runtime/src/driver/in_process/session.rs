use xylitol_plugin_sdk::error::XySessionError;
use xylitol_plugin_sdk::session::{
    SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel, plan_message_history_travel,
};

use super::XyDriver;
use super::XyDriverError;
use super::types::{
    SessionListEntry, SessionStats, estimate_from_session_entries, session_tree_kind_unimplemented,
};
use super::{bind_session_or_err, require_active_session};

impl super::XyInProcessDriver {
    pub async fn fork_session(
        &mut self,
        entry_id: &str,
        position: xylitol_plugin_sdk::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        let id = self
            .agent
            .fork_session(entry_id, position)
            .await
            .map_err(XyDriverError::from)?;
        self.bind_todo_session(Some(&id)).await;
        Ok(id)
    }

    pub async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        if !self.store.exists(session_id).await {
            return Err(XyDriverError::not_found(session_id.to_string()));
        }
        let (ty, phase, ctx) =
            crate::agent::runtime::script_hook_ctx::session_before_switch("resume", session_id);
        self.agent.script_hook_cancel(ty, phase, ctx).await?;
        let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_shutdown_resume(
            session_id,
            self.agent.session_id(),
        );
        self.agent.script_hook_observe(ty, phase, ctx).await;
        let context = self.store.build_session_context(session_id).await?;
        bind_session_or_err(&mut self.agent, session_id.to_string())?;
        self.bind_todo_session(Some(session_id)).await;
        self.agent.restore_thinking_level(context.thinking_level);
        self.agent.clear_tool_freeze();
        self.fire_session_prepare(xylitol_plugin_sdk::SessionPrepareReason::Resume);
        self.agent.emit_leaf_changed_settlement().await;
        Ok(session_id.to_string())
    }

    pub async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        self.store.load_entries(sid).await.map_err(Into::into)
    }

    pub async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        self.agent
            .get_session_stats()
            .await
            .map_err(XyDriverError::from)
    }

    pub async fn estimate_context_tokens(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
        let entries = self.get_messages().await?;
        let model_id = self.current_model().map(|m| m.id);
        let fixed_context = self.agent.fixed_request_context();
        tokio::task::spawn_blocking(move || {
            estimate_from_session_entries(&entries, model_id, Some(fixed_context))
        })
        .await
        .map_err(|e| XyDriverError::io(format!("estimate join: {e}")))
    }

    pub async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        let kind_s = format!("{kind:?}");
        let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_tree(&kind_s);
        self.agent.script_hook_cancel(ty, phase, ctx).await?;
        self.agent
            .ensure_session(sid, None)
            .await
            .map_err(XyDriverError::from)?;
        let tree = match kind {
            SessionTreeKind::MessageHistory => self.store.message_history_tree(sid).await?,
            SessionTreeKind::FileBrowser => {
                return Err(XyDriverError::unsupported(session_tree_kind_unimplemented(
                    kind,
                )));
            }
        };
        let kind_s = format!("{kind:?}");
        let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_tree(&kind_s);
        self.agent.script_hook_observe(ty, phase, ctx).await;
        Ok(tree)
    }

    pub async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        let kind_s = format!("{kind:?}");
        let (ty, phase, ctx) =
            crate::agent::runtime::script_hook_ctx::session_before_tree_travel(&kind_s, entry_id);
        self.agent.script_hook_cancel(ty, phase, ctx).await?;
        let travel = match kind {
            SessionTreeKind::MessageHistory => {
                let entries = self.store.load_entries(sid).await?;
                let travel = plan_message_history_travel(&entries, entry_id)?;
                self.store.set_leaf(sid, travel.leaf_id.as_deref());
                travel
            }
            SessionTreeKind::FileBrowser => {
                return Err(XyDriverError::unsupported(session_tree_kind_unimplemented(
                    kind,
                )));
            }
        };
        let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_tree_travel(
            &kind_s,
            entry_id,
            travel.leaf_id.as_deref(),
        );
        self.agent.script_hook_observe(ty, phase, ctx).await;
        Ok(travel)
    }

    pub async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        use xylitol_plugin_sdk::session::{EntryBase, LabelEntry};

        let sid = require_active_session(&self.agent)?;
        self.agent
            .ensure_session(sid, None)
            .await
            .map_err(XyDriverError::from)?;
        let entries = self.store.load_entries(sid).await?;
        if !entries.iter().any(|e| e.entry_id() == Some(target_id)) {
            return Err(XySessionError::entry_not_found(target_id).into());
        }
        let cleaned = label
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let entry = SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: "label".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            target_id: target_id.to_string(),
            label: cleaned,
        });
        self.store
            .append_session_entry(sid, &entry)
            .await
            .map_err(Into::into)
    }

    pub fn leaf_entry_id(&self) -> Option<String> {
        let sid = self.agent.session_id()?;
        self.store.leaf_id(sid)
    }

    pub async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        self.store.list_sessions().await.map_err(Into::into)
    }

    pub async fn load_session_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        self.store
            .load_entries(session_id)
            .await
            .map_err(Into::into)
    }

    pub async fn new_session(&mut self) -> Result<String, XyDriverError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        self.store
            .create(&session_id, cwd.as_deref(), None)
            .await
            .map_err(XyDriverError::from)?;
        bind_session_or_err(&mut self.agent, session_id.clone())?;
        self.bind_todo_session(Some(&session_id)).await;
        Ok(session_id)
    }

    pub async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        self.store.get_session_name(sid).await.map_err(Into::into)
    }

    pub async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        let out = self.store.set_session_name(sid, name).await?;
        self.agent.set_session_name_fact(Some(out.clone()));
        xylitol_plugin_sdk::set_obs_session_name(Some(out.as_str()));
        Ok(out)
    }

    pub async fn set_session_name_for(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XyDriverError> {
        let out = self.store.set_session_name(session_id, name).await?;
        if self.agent.session_id() == Some(session_id) {
            self.agent.set_session_name_fact(Some(out.clone()));
            xylitol_plugin_sdk::set_obs_session_name(Some(out.as_str()));
        }
        Ok(out)
    }

    pub async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        self.store
            .delete_session(session_id)
            .await
            .map_err(Into::into)
    }
}
