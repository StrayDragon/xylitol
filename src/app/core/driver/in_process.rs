//! In-process [`XyInProcessDriver`].

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agent::AgentRuntime;
use crate::protocol::ports::{XyBashResult, XySessionStore};
use crate::protocol::session::{
    SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel, plan_message_history_travel,
};
use crate::protocol::types::ThinkingLevel;

use super::XyDriver;
use super::XyDriverError;
use super::types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    ModelInfo, ProjectTrustMode, ProjectTrustPersistReport, ReloadStepReport, RuntimeReloadReport,
    SessionListEntry, SessionStats, estimate_from_session_entries, estimate_opts_from_app_config,
    session_tree_kind_unimplemented, tokenizer_override_from_app_config,
};

// ── In-process driver ─────────────────────────────────────────────

struct InProcessReloadState {
    cwd: std::path::PathBuf,
    agent_dir: std::path::PathBuf,
    project_trusted: bool,
    mcp: crate::app::core::composition::McpSession,
    mcp_servers: Vec<crate::app::core::mcp_spec::McpServerSpec>,
}

/// In-process driver wrapping the local agent module.
///
/// Constructed at the composition root (`app::cli` via `bootstrap`) which wires
/// ports and agent together. This is the **only** place in the app surfaces
/// that imports `agent`.
pub struct XyInProcessDriver {
    agent: AgentRuntime,
    /// Session store, held so SwitchSession/GetMessages/Fork can operate. The
    /// agent holds its own clone internally; this one is the surface's handle
    /// for session-management commands.
    store: Arc<dyn XySessionStore>,
    /// Optional reload state for `/reload` and MCP ownership (c1120).
    reload: Option<InProcessReloadState>,
}

impl XyInProcessDriver {
    fn map_str<T>(r: Result<T, String>) -> Result<T, XyDriverError> {
        r.map_err(XyDriverError::from_opaque)
    }

    /// Construct from a built agent plus the store used to build it.
    ///
    /// `store` is the same instance injected into the agent at construction;
    /// holding it here lets session commands operate without reaching into
    /// agent internals.
    pub fn new(agent: AgentRuntime, store: Arc<dyn XySessionStore>) -> Self {
        Self {
            agent,
            store,
            reload: None,
        }
    }

    /// Enable `/reload` and MCP ownership for the process lifetime (c1120).
    pub fn enable_reload_state(
        &mut self,
        cwd: std::path::PathBuf,
        agent_dir: std::path::PathBuf,
        project_trusted: bool,
        mcp_servers: Vec<crate::app::core::mcp_spec::McpServerSpec>,
    ) {
        self.reload = Some(InProcessReloadState {
            cwd,
            agent_dir,
            project_trusted,
            mcp: crate::app::core::composition::McpSession::new(),
            mcp_servers,
        });
    }

    /// Initial MCP bootstrap after assembly (cli / server). No-op when reload disabled.
    pub async fn bootstrap_mcp(&mut self) -> Result<(), XyDriverError> {
        let servers = self
            .reload
            .as_ref()
            .map(|s| s.mcp_servers.clone())
            .unwrap_or_default();
        self.reload_mcp_with_servers(&servers).await
    }

    /// One-line MCP status for startup logs (empty when reload/MCP disabled).
    pub async fn mcp_status_summary(&self) -> Option<String> {
        let state = self.reload.as_ref()?;
        if state.mcp_servers.is_empty() {
            return None;
        }
        let connected = state.mcp.connected_servers().await;
        let diags = state.mcp.diagnostics().await;
        let ids: Vec<_> = connected.iter().map(|s| s.id.as_str()).collect();
        let mut line = format!(
            "MCP: {} configured, {} connected [{}]",
            state.mcp_servers.len(),
            connected.len(),
            ids.join(", ")
        );
        if !diags.is_empty() {
            let detail = diags
                .iter()
                .map(|d| format!("{}: {}", d.server, d.message))
                .collect::<Vec<_>>()
                .join("; ");
            line.push_str(&format!("; diagnostics: {detail}"));
        }
        Some(line)
    }

    async fn reload_mcp_with_servers(
        &mut self,
        servers: &[crate::app::core::mcp_spec::McpServerSpec],
    ) -> Result<(), XyDriverError> {
        let Some(mut state) = self.reload.take() else {
            return Ok(());
        };
        let result = state.mcp.reload(self, servers).await;
        self.reload = Some(state);
        result
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.agent.cancel_token()
    }

    /// Replace the tool set (next `run`). Used by composition MCP reload.
    pub fn set_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.agent.set_tools(tools);
    }

    /// Replace context / SYSTEM / APPEND for the next `run` (c1100).
    /// Does not mutate session history.
    pub fn apply_prompt_resources(
        &mut self,
        context_files: Vec<(String, String)>,
        system_prompt: Option<String>,
        append_system_prompt: Vec<String>,
    ) {
        self.agent
            .apply_prompt_resources(context_files, system_prompt, append_system_prompt);
    }

    /// Replace skills catalog for the next `run` (c1085). Does not mutate history.
    pub fn apply_skills(&mut self, skills: Vec<crate::protocol::resource::SkillInfo>) {
        self.agent.apply_skills(skills);
    }

    /// Names currently in the system `<available_skills>` catalog (c1085).
    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.agent.loaded_skill_names()
    }

    /// Full skill catalog for `$` completion / expand (c1130).
    pub fn loaded_skills(&self) -> &[crate::protocol::resource::SkillInfo] {
        self.agent.loaded_skills()
    }

    fn skill_catalog_pairs(&self) -> Vec<(String, String)> {
        self.agent
            .loaded_skills()
            .iter()
            .map(|s| (s.name.clone(), s.description.clone().unwrap_or_default()))
            .collect()
    }

    /// Test/diagnostics: tool names currently registered.
    #[cfg(test)]
    pub(crate) fn tool_names_for_test(&self) -> Vec<String> {
        self.agent
            .inner()
            .tools()
            .iter()
            .map(|t| t.name().to_string())
            .collect()
    }

    /// Test/diagnostics: assembled system prompt text (c1100).
    #[cfg(test)]
    pub(crate) fn system_prompt_for_test(&self) -> Option<String> {
        self.agent.inner().system_prompt().map(String::from)
    }
}

#[async_trait]
impl XyDriver for XyInProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let sid = self
            .agent
            .inner()
            .session_id()
            .map(String::from)
            .unwrap_or_else(|| {
                let id = uuid::Uuid::new_v4().to_string();
                self.agent.inner_mut().set_session(id.clone());
                id
            });
        let stream = self.agent.run_with_id(prompt, &sid).await;
        Box::pin(stream)
    }

    fn abort(&self) {
        self.agent.abort();
    }

    fn current_model(&self) -> Option<ModelInfo> {
        self.agent
            .inner()
            .current_model()
            .map(|m| ModelInfo::from(&m))
    }

    fn active_turn(&self) -> Option<(String, ThinkingLevel, bool)> {
        let binding = self.agent.inner().inflight_turn_binding()?;
        Some((
            binding.display_name,
            binding.thinking,
            binding.omit_thinking,
        ))
    }

    fn has_active_turn(&self) -> bool {
        self.agent.inner().has_active_turn()
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        self.agent
            .inner()
            .model_registry()
            .list()
            .iter()
            .map(ModelInfo::from)
            .collect()
    }

    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        // Match by exact id or by config.model alias.
        let registry = self.agent.inner().model_registry();
        let found = registry
            .list()
            .iter()
            .find(|m| m.config.model == model_id || m.id == model_id)
            .map(|m| m.id.clone())
            .ok_or_else(|| XyDriverError::not_found(format!("model not found: {model_id}")))?;
        self.agent
            .inner_mut()
            .select_model(&found)
            .map_err(XyDriverError::from)?;
        // Re-read the resolved model to return authoritative info.
        Ok(self
            .agent
            .inner()
            .current_model()
            .map(|m| ModelInfo::from(&m))
            .unwrap_or_else(|| ModelInfo {
                id: found.clone(),
                display_name: found,
                thinking: true,
                thinking_levels: Vec::new(),
                context_window: 0,
            }))
    }

    fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
        let list = self.agent.inner().model_registry().list().to_vec();
        if list.is_empty() {
            return Err(XyDriverError::not_found("no models available"));
        }
        let current_id = self.agent.inner().current_model().map(|m| m.id.clone());
        let current_idx = current_id
            .as_ref()
            .and_then(|cur| list.iter().position(|m| m.id == *cur))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % list.len();
        let next_id = list[next_idx].id.clone();
        self.agent
            .inner_mut()
            .select_model_with_source(&next_id, "cycle")
            .map_err(XyDriverError::from)?;
        Ok(ModelInfo::from(&list[next_idx]))
    }

    fn set_thinking_level(&mut self, level: ThinkingLevel) -> Result<(), XyDriverError> {
        Self::map_str(self.agent.inner_mut().set_thinking_level(level))
    }

    fn thinking_level(&self) -> ThinkingLevel {
        self.agent.inner().thinking_level()
    }

    fn cycle_thinking_level(&mut self) -> Result<ThinkingLevel, XyDriverError> {
        Self::map_str(self.agent.inner_mut().cycle_thinking_level())
    }

    fn session_id(&self) -> Option<String> {
        self.agent.inner().session_id().map(String::from)
    }

    async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, XyDriverError> {
        Self::map_str(
            self.agent
                .inner()
                .execute_bash(command, exclude_from_context, chunk_tx)
                .await,
        )
    }

    async fn compact(&mut self) -> Result<bool, XyDriverError> {
        let model_id = self.current_model().map(|m| m.id);
        let opts = estimate_opts_from_app_config(model_id);
        Self::map_str(self.agent.inner_mut().maybe_auto_compact_with(&opts).await)
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        self.agent
            .inner_mut()
            .export_to_html(path)
            .await
            .map_err(XyDriverError::from)?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        self.agent
            .inner_mut()
            .export_to_jsonl(path)
            .await
            .map_err(XyDriverError::from)?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        Self::map_str(self.agent.inner_mut().import_from_jsonl(path).await)
    }

    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        Self::map_str(
            self.agent
                .inner_mut()
                .fork_session(entry_id, position)
                .await,
        )
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        if !self.store.exists(session_id).await {
            return Err(XyDriverError::not_found(format!(
                "session not found: {session_id}"
            )));
        }
        if let Some(bus) = self.agent.inner().hook_bus() {
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_switch("resume", session_id);
            crate::agent::session::cancel_hook(&bus, ty, phase, ctx).await?;
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_shutdown_resume(
                session_id,
                self.agent.inner().session_id(),
            );
            crate::agent::session::observe_hook(&bus, ty, phase, ctx).await;
        }
        self.agent.inner_mut().set_session(session_id.to_string());
        Ok(session_id.to_string())
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        let sid = self
            .agent
            .inner()
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        Self::map_str(self.store.load_entries(sid).await)
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        Self::map_str(self.agent.inner().get_session_stats().await)
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::types::ContextTokenEstimate, XyDriverError> {
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

    fn get_commands(&self) -> Vec<CommandInfo> {
        self.agent
            .inner()
            .get_commands()
            .into_iter()
            .map(|c| CommandInfo {
                name: c.name,
                description: c.description,
            })
            .collect()
    }

    fn steer(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.agent.steer(message);
        Ok(())
    }

    fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.agent.follow_up(message);
        Ok(())
    }

    fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        self.agent.clear_queues(clear_steer, clear_follow_up);
        Ok(())
    }

    fn queue_stats(&self) -> crate::agent::session::QueueStats {
        self.agent.queue_stats()
    }

    async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        let sid = self
            .agent
            .inner()
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        if let Some(bus) = self.agent.inner().hook_bus() {
            let kind = format!("{kind:?}");
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_tree(&kind);
            crate::agent::session::cancel_hook(&bus, ty, phase, ctx).await?;
        }
        // Bootstrap may assign a fresh id before any persist; wiped HOME may leave
        // an orphan id. Ensure an empty session so double-Esc opens an empty tree.
        self.agent.inner().ensure_session(sid, None).await?;
        let tree = match kind {
            SessionTreeKind::MessageHistory => self.store.message_history_tree(sid).await?,
            SessionTreeKind::FileBrowser => {
                return Err(XyDriverError::unsupported(session_tree_kind_unimplemented(
                    kind,
                )));
            }
        };
        if let Some(bus) = self.agent.inner().hook_bus() {
            let kind = format!("{kind:?}");
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_tree(&kind);
            crate::agent::session::observe_hook(&bus, ty, phase, ctx).await;
        }
        Ok(tree)
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        let sid = self
            .agent
            .inner()
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        if let Some(bus) = self.agent.inner().hook_bus() {
            let kind_s = format!("{kind:?}");
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_tree_travel(
                    &kind_s, entry_id,
                );
            crate::agent::session::cancel_hook(&bus, ty, phase, ctx).await?;
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
        if let Some(bus) = self.agent.inner().hook_bus() {
            let kind_s = format!("{kind:?}");
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_tree_travel(
                &kind_s,
                entry_id,
                travel.leaf_id.as_deref(),
            );
            crate::agent::session::observe_hook(&bus, ty, phase, ctx).await;
        }
        Ok(travel)
    }

    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        use crate::protocol::session::{EntryBase, LabelEntry};

        let sid = self
            .agent
            .inner()
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        self.agent.inner().ensure_session(sid, None).await?;
        let entries = self.store.load_entries(sid).await?;
        if !entries.iter().any(|e| e.entry_id() == Some(target_id)) {
            return Err(XyDriverError::not_found(format!(
                "target entry not found: {target_id}"
            )));
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
                timestamp: String::new(),
            },
            target_id: target_id.to_string(),
            label: cleaned,
        });
        Self::map_str(self.store.append_session_entry(sid, &entry).await)
    }

    fn leaf_entry_id(&self) -> Option<String> {
        let sid = self.agent.inner().session_id()?;
        self.store.leaf_id(sid)
    }

    async fn load_debug_scene(&mut self, scene: &str) -> Result<DebugSceneLoad, XyDriverError> {
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
        let canonical = seed_scene(self.store.as_ref(), &session_id, scene)
            .await
            .map_err(XyDriverError::from)?;
        self.agent.inner_mut().set_session(session_id.clone());
        let entries = self
            .store
            .load_entries(&session_id)
            .await
            .map_err(XyDriverError::from)?;
        let mut note = format!("debug scene `{canonical}` → session {session_id}");
        let model = match self.select_model("fake") {
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

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        Self::map_str(self.store.list_sessions().await)
    }

    async fn new_session(&mut self) -> Result<String, XyDriverError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        self.store
            .create(&session_id, cwd.as_deref(), None)
            .await
            .map_err(XyDriverError::from)?;
        self.agent.inner_mut().set_session(session_id.clone());
        Ok(session_id)
    }

    async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        let sid = self
            .agent
            .inner()
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        Self::map_str(self.store.get_session_name(sid).await)
    }

    async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
        let sid = self
            .agent
            .inner()
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        Self::map_str(self.store.set_session_name(sid, name).await)
    }

    async fn set_session_name_for(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XyDriverError> {
        Self::map_str(self.store.set_session_name(session_id, name).await)
    }

    async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        Self::map_str(self.store.delete_session(session_id).await)
    }

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        self.skill_catalog_pairs()
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        let skill_names = self.loaded_skill_names();
        let Some(state) = self.reload.as_ref() else {
            return LoadedResourcesSnapshot {
                skill_names,
                ..LoadedResourcesSnapshot::default()
            };
        };
        let connected = state.mcp.connected_servers().await;
        let diags = state.mcp.diagnostics().await;
        LoadedResourcesSnapshot {
            skill_names,
            mcp_connected: connected
                .into_iter()
                .map(|s| (s.id, s.tool_count))
                .collect(),
            mcp_configured: state.mcp_servers.len(),
            mcp_diag_short: diags
                .into_iter()
                .map(|d| format!("{}: {}", d.server, d.message))
                .collect(),
        }
    }

    async fn reload_runtime(&mut self) -> Result<RuntimeReloadReport, XyDriverError> {
        let Some(mut state) = self.reload.take() else {
            return Ok(RuntimeReloadReport::noop());
        };

        let mut steps = Vec::new();

        // Re-read trust store so `/trust` + later `/reload` picks up new decisions (c1105).
        let trust_mgr = crate::infra::trust::TrustManager::new(
            crate::infra::trust::TrustManager::default_dir(),
        );
        let cwd_str = state.cwd.display().to_string();
        state.project_trusted = trust_mgr.is_trusted(&cwd_str);

        let app_config = crate::infra::config::loader::load_app_config(None).ok();
        state.mcp_servers = crate::app::core::mcp_spec::McpServerSpec::from_infra_list(
            app_config.as_ref().and_then(|c| c.mcp_servers.clone()),
        )
        .unwrap_or_default();
        let config_system_prompt = app_config
            .as_ref()
            .and_then(|cfg| cfg.resolve_default_profile().ok())
            .and_then(|p| p.system_prompt.clone());

        let skills = crate::app::core::bootstrap::reload_skills(
            self,
            &state.cwd,
            &state.agent_dir,
            state.project_trusted,
        );
        let skills_msg = if skills.names.is_empty() {
            "0 skills".into()
        } else {
            format!("{} skill(s): {}", skills.count, skills.names.join(", "))
        };
        steps.push(ReloadStepReport {
            step: "skills",
            ok: true,
            message: skills_msg,
        });

        match state.mcp.reload(self, &state.mcp_servers).await {
            Ok(()) => {
                let connected = state.mcp.connected_servers().await;
                let diags = state.mcp.diagnostics().await;
                if diags.is_empty() {
                    let ids: Vec<_> = connected.iter().map(|s| s.id.as_str()).collect();
                    steps.push(ReloadStepReport {
                        step: "mcp",
                        ok: true,
                        message: format!(
                            "{} configured, {} connected [{}]",
                            state.mcp_servers.len(),
                            connected.len(),
                            ids.join(", ")
                        ),
                    });
                } else {
                    let detail = diags
                        .iter()
                        .map(|d| format!("{}: {}", d.server, d.message))
                        .collect::<Vec<_>>()
                        .join("; ");
                    steps.push(ReloadStepReport {
                        step: "mcp",
                        ok: !connected.is_empty(),
                        message: format!(
                            "{} configured, {} connected; diagnostics: {detail}",
                            state.mcp_servers.len(),
                            connected.len()
                        ),
                    });
                }
            }
            Err(e) => steps.push(ReloadStepReport {
                step: "mcp",
                ok: false,
                message: e.to_string(),
            }),
        }

        let ctx = crate::app::core::bootstrap::reload_prompt_context(
            self,
            &state.cwd,
            &state.agent_dir,
            state.project_trusted,
            config_system_prompt,
        );
        steps.push(ReloadStepReport {
            step: "context",
            ok: true,
            message: format!(
                "{} context file(s), system={}, append={}",
                ctx.context_file_count, ctx.has_system_prompt, ctx.append_count
            ),
        });

        self.reload = Some(state);
        Ok(RuntimeReloadReport { steps })
    }

    fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        let cwd = self
            .reload
            .as_ref()
            .map(|s| s.cwd.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let cwd_str = cwd.display().to_string();
        let mgr = crate::infra::trust::TrustManager::new(
            crate::infra::trust::TrustManager::default_dir(),
        );
        let options = mgr.get_trust_options(&cwd_str, false);
        let opt = match mode {
            ProjectTrustMode::TrustCwd => options.first(),
            ProjectTrustMode::TrustParent => {
                options.iter().find(|o| o.label.starts_with("Trust parent"))
            }
            ProjectTrustMode::Deny => options.iter().find(|o| o.label == "Do not trust"),
        };
        let Some(opt) = opt else {
            return Err(match mode {
                ProjectTrustMode::TrustParent => {
                    XyDriverError::invalid_input("no parent folder to trust")
                }
                _ => XyDriverError::invalid_input("trust option unavailable"),
            });
        };
        if !opt.updates.is_empty() {
            mgr.apply_updates(&opt.updates)
                .map_err(|e| XyDriverError::io(format!("trust store write failed: {e}")))?;
        }
        let saved = opt.saved_path.clone().unwrap_or_else(|| cwd_str.clone());
        let verb = if opt.trusted { "trusted" } else { "denied" };
        Ok(ProjectTrustPersistReport {
            trusted: opt.trusted,
            saved_path: opt.saved_path.clone(),
            message: format!(
                "Project {verb} at {saved}. {}",
                ProjectTrustPersistReport::RELOAD_HINT
            ),
        })
    }

    async fn copy_text_to_clipboard(
        &mut self,
        text: &str,
    ) -> Result<ClipboardCopyOutcome, XyDriverError> {
        let plan = crate::infra::clipboard::plan_clipboard_copy_async(text.to_string())
            .await
            .map_err(XyDriverError::io)?;
        if !plan.will_succeed() {
            return Err(XyDriverError::io(plan.failure_message()));
        }
        Ok(ClipboardCopyOutcome {
            pending_osc52: plan.osc52_sequence,
        })
    }

    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, XyDriverError> {
        let image = tokio::task::spawn_blocking(crate::infra::clipboard::read_clipboard_image)
            .await
            .map_err(|e| XyDriverError::io(format!("clipboard image task failed: {e}")))?
            .map_err(XyDriverError::io)?;
        let Some(image) = image else {
            return Ok(None);
        };
        let path = tokio::task::spawn_blocking(move || {
            crate::infra::clipboard::write_clipboard_image_temp(&image.bytes, &image.mime_type)
        })
        .await
        .map_err(|e| XyDriverError::io(format!("clipboard image write task failed: {e}")))?
        .map_err(XyDriverError::io)?;
        Ok(Some(path))
    }

    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        tokio::task::spawn_blocking(crate::infra::clipboard::read_clipboard_text)
            .await
            .map_err(|e| XyDriverError::io(format!("clipboard text task failed: {e}")))?
            .map_err(XyDriverError::io)
    }
}

#[cfg(test)]
mod driver_session_tree_tests {
    use std::sync::Arc;

    use super::*;
    use crate::agent::AgentBuilder;
    use crate::agent::tools::ToolSet;
    use crate::infra::bash_exec::InfraBashExecutor;
    use crate::infra::config::value::InfraSecretResolver;
    use crate::infra::event::EventBus;
    use crate::infra::export::StdExportIo;
    use crate::infra::permission;
    use crate::infra::session::SessionManager;
    use crate::protocol::model_config::XyModelConfig;
    use crate::protocol::ports::{
        XyBashExecutor, XyEventSink, XyExportIo, XyModel, XySessionStore,
    };
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry, SessionTreeKind};

    type ModelBuilderFn =
        Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>;

    fn msg_entry(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: format!("2026-01-01T00:00:00.{id}Z"),
            },
            message: crate::protocol::session::fixture_message_json(role, text),
        })
    }

    async fn build_test_driver(store: Arc<SessionManager>) -> XyInProcessDriver {
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut agent = AgentBuilder::new(
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new())),
            Arc::new(crate::infra::provider::factory::build_provider),
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
        .bash(Arc::new(InfraBashExecutor::new()) as Arc<dyn XyBashExecutor>)
        .export_io(Arc::new(StdExportIo::new()) as Arc<dyn XyExportIo>)
        .build()
        .expect("build agent");
        let sid = uuid::Uuid::new_v4().to_string();
        store_trait
            .create(&sid, Some("."), None)
            .await
            .expect("create session");
        agent.inner_mut().set_session(sid);
        XyInProcessDriver::new(agent, store)
    }

    #[tokio::test]
    async fn in_process_session_tree_ensures_missing_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut agent = AgentBuilder::new(
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new())),
            Arc::new(crate::infra::provider::factory::build_provider),
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
        .bash(Arc::new(InfraBashExecutor::new()) as Arc<dyn XyBashExecutor>)
        .export_io(Arc::new(StdExportIo::new()) as Arc<dyn XyExportIo>)
        .build()
        .expect("build agent");
        // Orphan id: set on agent but never created on disk (wipe / pre-persist).
        let orphan = uuid::Uuid::new_v4().to_string();
        agent.inner_mut().set_session(orphan.clone());
        let driver = XyInProcessDriver::new(agent, store);
        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("empty tree after ensure");
        assert!(tree.is_empty(), "expected empty tree, got {tree:?}");
        assert!(
            store_trait.exists(&orphan).await,
            "ensure_session must create orphan session"
        );
    }

    #[tokio::test]
    async fn in_process_session_tree_returns_parent_child() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "hello"))
            .await
            .expect("append u1");
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "hi"))
            .await
            .expect("append a1");

        let loaded = store.load(&sid).await.expect("load after append");
        let ids: Vec<_> = loaded.iter().filter_map(|e| e.entry_id()).collect();
        assert_eq!(
            ids,
            vec!["u1", "a1"],
            "persisted entries before tree: {loaded:?}"
        );

        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("tree");
        assert_eq!(tree.len(), 1, "tree={tree:?} loaded={loaded:?}");
        assert_eq!(tree[0].entry.entry_id(), Some("u1"));
        assert_eq!(
            tree[0].children.len(),
            1,
            "expected a1 under u1; tree={tree:?} loaded={loaded:?}"
        );
        assert_eq!(tree[0].children[0].entry.entry_id(), Some("a1"));
    }

    #[tokio::test]
    async fn in_process_travel_user_sets_parent_leaf_and_editor_text() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "edit me"))
            .await
            .unwrap();
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "reply"))
            .await
            .unwrap();
        XySessionStore::set_leaf(store.as_ref(), &sid, Some("a1"));

        let travel = driver
            .travel_session_tree(SessionTreeKind::MessageHistory, "u1")
            .await
            .expect("travel");
        assert_eq!(travel.leaf_id, None);
        assert_eq!(travel.editor_text.as_deref(), Some("edit me"));
        assert_eq!(XySessionStore::leaf_id(store.as_ref(), &sid), None);
    }

    #[tokio::test]
    async fn in_process_travel_non_user_sets_leaf_without_editor_text() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "hello"))
            .await
            .unwrap();
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "reply"))
            .await
            .unwrap();

        let travel = driver
            .travel_session_tree(SessionTreeKind::MessageHistory, "a1")
            .await
            .expect("travel");
        assert_eq!(travel.leaf_id.as_deref(), Some("a1"));
        assert!(travel.editor_text.is_none());
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("a1")
        );
    }

    #[tokio::test]
    async fn unsupported_tree_kind_returns_err_without_changing_leaf() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");
        XySessionStore::set_leaf(store.as_ref(), &sid, Some("keep"));

        let err = driver
            .session_tree(SessionTreeKind::FileBrowser)
            .await
            .expect_err("file_browser");
        assert!(err.to_string().contains("file_browser"));
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("keep")
        );

        let err = driver
            .travel_session_tree(SessionTreeKind::FileBrowser, "x")
            .await
            .expect_err("travel file_browser");
        assert!(err.to_string().contains("file_browser"));
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("keep")
        );
    }

    #[tokio::test]
    async fn after_run_session_tree_reflects_persisted_turn() {
        use std::pin::Pin;

        use async_trait::async_trait;
        use futures::StreamExt;

        use crate::protocol::error::XyError;
        use crate::protocol::message::XyStopReason;
        use crate::protocol::model_config::XyModelConfig;
        use crate::protocol::ports::{XyModel, XyStream};
        use crate::protocol::types::{XyChunk, XyModelMeta, XyToolSchema};

        struct TextMockModel;
        #[async_trait]
        impl XyModel for TextMockModel {
            fn name(&self) -> &str {
                "text-mock"
            }

            async fn generate_stream(
                &self,
                _messages: Vec<crate::protocol::message::LlmMessage>,
                _tools: &[XyToolSchema],
                _stream: bool,
                _options: crate::protocol::ports::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                let chunks = vec![
                    Ok(XyChunk::TextDelta("reply".into())),
                    Ok(XyChunk::Done {
                        finish_reason: XyStopReason::Stop,
                        usage: None,
                    }),
                ];
                Ok(Box::pin(futures::stream::iter(chunks))
                    as Pin<
                        Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>,
                    >)
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut reg =
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: XyModelConfig {
                kind: crate::protocol::model_config::XyModelKind::Fake,
                api_key: String::new(),
                model: "mock".into(),
                base_url: None,
                api: None,
            },
            display_name: "Mock".into(),
            thinking: false,
            context_window: 128000,
            api: String::new(),
            provider: String::new(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
            thinking_level_map: Default::default(),
        });
        let builder: ModelBuilderFn = Arc::new(|_| Ok(Arc::new(TextMockModel) as Arc<dyn XyModel>));
        let mut agent = AgentBuilder::new(
            reg,
            builder,
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn crate::protocol::ports::XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::empty())
        .bash(Arc::new(InfraBashExecutor::new()) as Arc<dyn XyBashExecutor>)
        .export_io(Arc::new(StdExportIo::new()) as Arc<dyn XyExportIo>)
        .build()
        .expect("build agent");
        agent.inner_mut().select_model("mock").expect("select mock");
        let sid = uuid::Uuid::new_v4().to_string();
        agent.inner_mut().set_session(sid);
        let mut driver = XyInProcessDriver::new(agent, store_trait);

        let mut stream = driver.run("hello tree").await;
        while stream.next().await.is_some() {}

        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("tree");
        assert!(
            !tree.is_empty(),
            "session tree should reflect persisted messages"
        );
    }

    #[tokio::test]
    async fn fork_rejects_unflushed_session_via_driver() {
        // TUI cannot hit this while assistant is streaming (steer takes over); cover via XyDriver.
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");
        let path = store.get_session_file(&sid).expect("persisted path");
        assert!(
            !path.exists(),
            "build_test_driver leaves session pending-only"
        );

        store
            .append(
                &sid,
                &SessionEntry::Message(MessageEntry {
                    base: EntryBase {
                        entry_type: "message".into(),
                        id: String::new(),
                        parent_id: None,
                        timestamp: String::new(),
                    },
                    message: crate::protocol::session::fixture_message_json("user", "only user"),
                }),
            )
            .await
            .expect("pending user");
        assert!(!path.exists());

        let uid = store
            .load(&sid)
            .await
            .expect("load")
            .iter()
            .find_map(|e| e.entry_id())
            .expect("user id")
            .to_string();

        let err = driver
            .fork_session(&uid, crate::protocol::session::ForkPosition::At)
            .await
            .expect_err("unflushed fork");
        assert!(
            err.to_string().contains("not been saved yet"),
            "pi unflushed guard via XyDriver: {err}"
        );
    }

    #[tokio::test]
    async fn persist_project_trust_writes_store_under_home() {
        let home = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", home.path());
        }
        let cwd = home.path().join("proj");
        std::fs::create_dir_all(cwd.join(".xylitol")).unwrap();
        let store = Arc::new(SessionManager::new(home.path().join("sessions")));
        let mut driver = build_test_driver(store).await;
        driver.enable_reload_state(cwd.clone(), home.path().join(".xylitol"), false, Vec::new());
        let report = driver
            .persist_project_trust(ProjectTrustMode::TrustCwd)
            .expect("persist");
        assert!(report.trusted);
        assert!(report.message.contains("/reload") || report.message.contains("restart"));
        let mgr = crate::infra::trust::TrustManager::new(
            crate::infra::trust::TrustManager::default_dir(),
        );
        assert!(mgr.is_trusted(&cwd.display().to_string()));
        match prev {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}
