use crate::protocol::error::XySessionError;
use crate::protocol::session::{
    SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel, plan_message_history_travel,
};

use super::XyDriver;
use super::XyDriverError;
use super::mcp::McpBootState;
use super::types::{
    DebugSceneLoad, SessionListEntry, SessionStats, estimate_from_session_entries,
    session_tree_kind_unimplemented, tokenizer_override_from_app_config,
};
use super::{bind_session_or_err, require_active_session};

impl super::XyInProcessDriver {
    pub(super) async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        let id = self
            .agent
            .fork_session(entry_id, position)
            .await
            .map_err(XyDriverError::from)?;
        self.bind_todo_session(Some(&id)).await;
        Ok(id)
    }

    pub(super) async fn switch_session(
        &mut self,
        session_id: &str,
    ) -> Result<String, XyDriverError> {
        if !self.store.exists(session_id).await {
            return Err(XyDriverError::not_found(session_id.to_string()));
        }
        if let Some(bus) = self.agent.hook_bus() {
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_switch("resume", session_id);
            crate::agent::capabilities::cancel_hook(&bus, ty, phase, ctx).await?;
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_shutdown_resume(
                session_id,
                self.agent.session_id(),
            );
            crate::agent::capabilities::observe_hook(&bus, ty, phase, ctx).await;
        }
        let context = self.store.build_session_context(session_id).await?;
        bind_session_or_err(&mut self.agent, session_id.to_string())?;
        self.bind_todo_session(Some(session_id)).await;
        // Restore precisely what the session recorded. Do not validate, clamp, or
        // append a replacement event: a stale vendor level is intentionally sticky
        // until the user changes or cycles it.
        self.agent.restore_thinking_level(context.thinking_level);
        // c1905: session_env lives in transcript (Env→user); resume appends a new
        // row only when date/cwd change — no system-date pin restore needed.
        // c1900: resume/switch re-opens the freeze gate — next generate re-gates.
        // Fingerprint match/continue-freeze needs persisted fingerprint (same change wave MAY
        // add Custom/header storage); until then correctness prefers re-freeze.
        self.agent.clear_tool_freeze();
        self.mcp_boot = McpBootState::Idle;
        if self
            .reload
            .as_ref()
            .is_some_and(|s| !s.mcp_servers.is_empty())
        {
            self.begin_mcp_bootstrap().await;
        } else {
            self.mcp_boot = McpBootState::Settled;
        }
        if let Ok(Some(name)) = self.store.get_session_name(session_id).await {
            xylitol_ai_bridge::provider::set_obs_session_name(Some(name.as_str()));
        }
        Ok(session_id.to_string())
    }

    pub(super) async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        self.store.load_entries(sid).await.map_err(Into::into)
    }

    pub(super) async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        self.agent
            .get_session_stats()
            .await
            .map_err(XyDriverError::from)
    }

    pub(super) async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::model::ContextTokenEstimate, XyDriverError> {
        let entries = self.get_messages().await?;
        let model_id = self.current_model().map(|m| m.id);
        let tokenizer_override = model_id
            .as_deref()
            .and_then(tokenizer_override_from_app_config);
        // HF / local encode is CPU-heavy — keep it off the async worker (TUI host loop).
        tokio::task::spawn_blocking(move || {
            estimate_from_session_entries(&entries, model_id, tokenizer_override)
        })
        .await
        .map_err(|e| XyDriverError::io(format!("estimate join: {e}")))
    }

    pub(super) async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        if let Some(bus) = self.agent.hook_bus() {
            let kind = format!("{kind:?}");
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_tree(&kind);
            crate::agent::capabilities::cancel_hook(&bus, ty, phase, ctx).await?;
        }
        // Bootstrap may assign a fresh id before any persist; wiped HOME may leave
        // an orphan id. Ensure an empty session so double-Esc opens an empty tree.
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
        if let Some(bus) = self.agent.hook_bus() {
            let kind = format!("{kind:?}");
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_tree(&kind);
            crate::agent::capabilities::observe_hook(&bus, ty, phase, ctx).await;
        }
        Ok(tree)
    }

    pub(super) async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        if let Some(bus) = self.agent.hook_bus() {
            let kind_s = format!("{kind:?}");
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_tree_travel(
                    &kind_s, entry_id,
                );
            crate::agent::capabilities::cancel_hook(&bus, ty, phase, ctx).await?;
        }
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
        if let Some(bus) = self.agent.hook_bus() {
            let kind_s = format!("{kind:?}");
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_tree_travel(
                &kind_s,
                entry_id,
                travel.leaf_id.as_deref(),
            );
            crate::agent::capabilities::observe_hook(&bus, ty, phase, ctx).await;
        }
        Ok(travel)
    }

    pub(super) async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        use crate::protocol::session::{EntryBase, LabelEntry};

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

    pub(super) fn leaf_entry_id(&self) -> Option<String> {
        let sid = self.agent.session_id()?;
        self.store.leaf_id(sid)
    }

    pub(super) async fn load_debug_scene(
        &mut self,
        scene: &str,
    ) -> Result<DebugSceneLoad, XyDriverError> {
        use crate::app::debug_fixtures::{list_note, resolve_scene_id, seed_scene};

        let scene = scene.trim();
        if scene.is_empty() || scene.eq_ignore_ascii_case("list") {
            return Err(XyDriverError::invalid_input(list_note()));
        }
        let canonical = resolve_scene_id(scene).ok_or_else(|| {
            XyDriverError::invalid_input(format!("unknown debug scene: {scene}\n{}", list_note()))
        })?;
        let short = &uuid::Uuid::new_v4().to_string()[..8];
        let session_id = format!("debug-{canonical}-{short}");
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        self.store
            .create(&session_id, cwd.as_deref(), None)
            .await
            .map_err(XyDriverError::from)?;
        let canonical = seed_scene(self.store.as_ref(), &session_id, scene).await?;
        bind_session_or_err(&mut self.agent, session_id.clone())?;
        self.bind_todo_session(Some(&session_id)).await;
        let entries = self
            .store
            .load_entries(&session_id)
            .await
            .map_err(XyDriverError::from)?;
        let mut note = format!("debug scene `{canonical}` → session {session_id}");
        let model = match self.select_model("fake").await {
            Ok(m) => {
                note.push_str("; model → fake");
                Some(m)
            }
            Err(_) => {
                note.push_str("; fake not in catalog (tree fixture only)");
                None
            }
        };
        Ok(DebugSceneLoad {
            session_id,
            entries,
            note,
            model,
        })
    }

    pub(super) async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        self.store.list_sessions().await.map_err(Into::into)
    }

    pub(super) async fn load_session_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        self.store
            .load_entries(session_id)
            .await
            .map_err(Into::into)
    }

    pub(super) async fn new_session(&mut self) -> Result<String, XyDriverError> {
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

    pub(super) async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        self.store.get_session_name(sid).await.map_err(Into::into)
    }

    pub(super) async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
        let sid = require_active_session(&self.agent)?;
        let out = self.store.set_session_name(sid, name).await?;
        xylitol_ai_bridge::provider::set_obs_session_name(Some(out.as_str()));
        Ok(out)
    }

    pub(super) async fn set_session_name_for(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XyDriverError> {
        let out = self.store.set_session_name(session_id, name).await?;
        if self.agent.session_id() == Some(session_id) {
            xylitol_ai_bridge::provider::set_obs_session_name(Some(out.as_str()));
        }
        Ok(out)
    }

    pub(super) async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        self.store
            .delete_session(session_id)
            .await
            .map_err(Into::into)
    }
}
