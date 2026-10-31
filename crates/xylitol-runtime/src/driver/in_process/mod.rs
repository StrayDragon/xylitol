use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agent::AgentRuntime;
use crate::agent::runtime::RunPolicy;
use crate::driver::bang_exec::BangExecHandler;
use crate::driver::session_export::SessionExporter;
use xylitol_plugin_sdk::AgentTodoGateway;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::error::XySessionError;
use xylitol_plugin_sdk::{PlanGateway, WorkflowGateway};

use super::types::{
    CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport,
};
pub(super) use super::{XyDriver, XyDriverError, types};

mod prepare;
mod session;

pub use prepare::InProcessReloadHook;

struct NoopTodo;

#[async_trait]
impl AgentTodoGateway for NoopTodo {
    async fn list(
        &self,
    ) -> Result<xylitol_plugin_sdk::session::TodoList, xylitol_plugin_sdk::error::XyToolError> {
        Ok(xylitol_plugin_sdk::session::TodoList { items: Vec::new() })
    }
    async fn rewrite(
        &self,
        _items: Vec<xylitol_plugin_sdk::session::TodoItemDraft>,
    ) -> Result<xylitol_plugin_sdk::session::TodoList, xylitol_plugin_sdk::error::XyToolError> {
        Ok(xylitol_plugin_sdk::session::TodoList { items: Vec::new() })
    }
    async fn update(
        &self,
        _patches: Vec<xylitol_plugin_sdk::session::TodoItemPatch>,
    ) -> Result<xylitol_plugin_sdk::session::TodoList, xylitol_plugin_sdk::error::XyToolError> {
        Ok(xylitol_plugin_sdk::session::TodoList { items: Vec::new() })
    }
}

pub struct XyInProcessDriver {
    pub agent: AgentRuntime,
    pub store: Arc<dyn XySessionStore>,
    bang: BangExecHandler,
    exporter: SessionExporter,
    ask_gateway: Option<Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>>,
    todo_gateway: Arc<dyn AgentTodoGateway>,
    plan_gateway: Option<Arc<dyn PlanGateway>>,
    workflow_gateway: Option<Arc<dyn WorkflowGateway>>,
    bash_run_sink: std::sync::Mutex<Option<xylitol_plugin_sdk::BashOutputSink>>,
    slash_commands: Vec<CommandInfo>,
    ctx: Option<xylitol_plugin_sdk::rt::Ctx>,
    prepare_hub: Option<Arc<dyn xylitol_plugin_sdk::SessionPrepareHub>>,
    reload_hook: Option<Arc<dyn InProcessReloadHook>>,
    reload_cwd: Option<std::path::PathBuf>,
    reload_agent_dir: Option<std::path::PathBuf>,
    reload_config_path: Option<std::path::PathBuf>,
    last_ui_mcp_label: Option<Option<String>>,
    mcp_gate_notice: Option<String>,
    prepare_acked: bool,
    base_tools: Option<crate::agent::tools::ToolSet>,
    capability_disabled: BTreeSet<String>,
    capability_applied: BTreeSet<String>,
    mcp_server_ids: Vec<String>,
    plugin_commands: Vec<(String, Arc<dyn xylitol_plugin_sdk::XyPluginCommand>)>,
}

impl XyInProcessDriver {
    pub fn new(agent: AgentRuntime, store: Arc<dyn XySessionStore>) -> Self {
        Self::with_surface_ops(
            agent,
            store,
            BangExecHandler::new(None),
            SessionExporter::new(None),
        )
    }

    pub fn with_surface_ops(
        agent: AgentRuntime,
        store: Arc<dyn XySessionStore>,
        bang: BangExecHandler,
        exporter: SessionExporter,
    ) -> Self {
        Self {
            agent,
            store,
            bang,
            exporter,
            ask_gateway: None,
            todo_gateway: Arc::new(NoopTodo),
            plan_gateway: None,
            workflow_gateway: None,
            bash_run_sink: std::sync::Mutex::new(None),
            slash_commands: Vec::new(),
            ctx: None,
            prepare_hub: None,
            reload_hook: None,
            reload_cwd: None,
            reload_agent_dir: None,
            reload_config_path: None,
            last_ui_mcp_label: None,
            mcp_gate_notice: None,
            prepare_acked: true,
            base_tools: None,
            capability_disabled: BTreeSet::new(),
            capability_applied: BTreeSet::new(),
            mcp_server_ids: Vec::new(),
            plugin_commands: Vec::new(),
        }
    }

    pub fn with_bang(
        agent: AgentRuntime,
        store: Arc<dyn XySessionStore>,
        bang: BangExecHandler,
    ) -> Self {
        Self::with_surface_ops(agent, store, bang, SessionExporter::new(None))
    }

    pub fn set_todo_gateway(&mut self, todo: Arc<dyn AgentTodoGateway>) {
        self.todo_gateway = todo;
    }

    pub fn set_plan_gateway(&mut self, plan: Arc<dyn PlanGateway>) {
        self.plan_gateway = Some(plan);
    }

    pub fn plan_gateway(&self) -> Option<Arc<dyn PlanGateway>> {
        self.plan_gateway.clone()
    }

    pub fn set_workflow_gateway(&mut self, workflow: Arc<dyn WorkflowGateway>) {
        self.workflow_gateway = Some(workflow);
    }

    pub fn workflow_gateway(&self) -> Option<Arc<dyn WorkflowGateway>> {
        self.workflow_gateway.clone()
    }

    pub fn set_plugin_commands(
        &mut self,
        commands: Vec<(String, Arc<dyn xylitol_plugin_sdk::XyPluginCommand>)>,
    ) {
        self.plugin_commands = commands;
    }

    pub fn plugin_command(
        &self,
        name: &str,
    ) -> Option<Arc<dyn xylitol_plugin_sdk::XyPluginCommand>> {
        if let Some((_, command)) = self.plugin_commands.iter().find(|(n, _)| n == name) {
            return Some(Arc::clone(command));
        }
        let registries = self
            .ctx
            .as_ref()?
            .get::<xylitol_plugin_sdk::HostRegistries>()?;
        registries.plugin_commands.get(name)
    }

    pub fn set_slash_commands(&mut self, commands: Vec<CommandInfo>) {
        self.slash_commands = commands;
    }

    pub fn install_ask_tool(&mut self, gateway: Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>) {
        self.ask_gateway = Some(gateway);
    }

    pub fn ask_gateway(&self) -> Option<Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>> {
        self.ask_gateway.clone()
    }

    async fn bind_todo_session(&self, session_id: Option<&str>) {
        let session_id = session_id.map(str::to_string);
        self.todo_gateway.bind_session(session_id.clone()).await;
        self.bind_optional_gateways(session_id).await;
    }

    async fn bind_optional_gateways(&self, session_id: Option<String>) {
        if let Some(plan) = self.plan_gateway.as_ref() {
            plan.bind_session(session_id.clone()).await;
        }
        if let Some(workflow) = self.workflow_gateway.as_ref() {
            workflow.bind_session(session_id).await;
        }
    }

    pub fn todo_gateway(&self) -> Arc<dyn AgentTodoGateway> {
        self.todo_gateway.clone()
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.agent.cancel_token()
    }

    pub fn set_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.agent.set_tools(tools);
    }

    pub fn set_base_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.base_tools = Some(tools);
    }

    pub fn set_mcp_server_ids(&mut self, ids: Vec<String>) {
        self.mcp_server_ids = ids;
    }

    pub fn capability_snapshot(&self) -> xylitol_plugin_sdk::session::CapabilitySnapshot {
        let base = self
            .base_tools
            .clone()
            .unwrap_or_else(|| self.agent.clone_tools());
        crate::agent::tools::capability_snapshot(
            &base,
            &self.mcp_server_ids,
            &self.capability_disabled,
            &self.capability_applied,
        )
    }

    pub fn capability_pending(&self) -> bool {
        self.capability_disabled != self.capability_applied
    }

    pub fn set_capability(
        &mut self,
        id: &str,
        enabled: bool,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        let snapshot = self.capability_snapshot();
        if !snapshot.entries.iter().any(|entry| entry.id == id) {
            return Err(XyDriverError::invalid_input(format!(
                "unknown capability `{id}`"
            )));
        }
        if enabled {
            self.capability_disabled.remove(id);
        } else {
            self.capability_disabled.insert(id.to_string());
        }
        Ok(self.capability_snapshot())
    }

    pub fn clear_capabilities(
        &mut self,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        self.capability_disabled.clear();
        Ok(self.capability_snapshot())
    }

    pub fn install_base_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.base_tools = Some(tools.clone());
        let filtered = crate::agent::tools::filter_tools_by_capability_overlay(
            tools,
            &self.capability_disabled,
        );
        self.agent.reopen_tools_for_regate();
        self.agent.freeze_tools(filtered);
        self.capability_applied = self.capability_disabled.clone();
    }

    fn apply_pending_capability_overlay(&mut self) {
        if !self.capability_pending() {
            return;
        }
        let base = self
            .base_tools
            .clone()
            .unwrap_or_else(|| self.agent.clone_tools());
        let filtered = crate::agent::tools::filter_tools_by_capability_overlay(
            base,
            &self.capability_disabled,
        );
        self.agent.reopen_tools_for_regate();
        self.agent.set_tools(filtered);
        self.capability_applied = self.capability_disabled.clone();
    }

    pub fn freeze_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.agent.freeze_tools(tools);
    }

    pub fn reopen_tools_for_regate(&mut self) {
        self.agent.reopen_tools_for_regate();
    }

    pub fn is_tools_frozen(&self) -> bool {
        self.agent.is_tools_frozen()
    }

    pub async fn ensure_tool_table_frozen(&mut self) {
        if self.agent.is_tools_frozen() {
            return;
        }
        self.agent.begin_tool_gating();
        let _ = self.join_session_prepare().await;
        let tools = self.agent.clone_tools();
        self.agent.freeze_tools(tools);
        self.prepare_acked = true;
    }

    pub fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        let cwd = self
            .reload_cwd
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| self.agent.cwd().to_string());
        let mgr = crate::trust::TrustManager::new(
            self.reload_agent_dir
                .clone()
                .unwrap_or_else(crate::trust::TrustManager::default_dir),
        );
        let options = mgr.get_trust_options(&cwd, false);
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
            mgr.apply_updates(&opt.updates)?;
        }
        let saved = opt.saved_path.clone().unwrap_or_else(|| cwd.clone());
        let verb = if opt.trusted { "trusted" } else { "denied" };
        Ok(ProjectTrustPersistReport {
            trusted: opt.trusted,
            saved_path: Some(saved),
            message: format!("Project {verb}. {}", ProjectTrustPersistReport::RELOAD_HINT),
        })
    }

    pub fn apply_prompt_resources(
        &mut self,
        context_files: Vec<(String, String)>,
        system_prompt: Option<String>,
        append_system_prompt: Vec<String>,
    ) {
        self.agent
            .apply_prompt_resources(context_files, system_prompt, append_system_prompt);
    }

    pub fn apply_skills(&mut self, skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>) {
        self.agent.apply_skills(skills);
    }

    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.agent.loaded_skill_names()
    }

    pub fn loaded_skills(&self) -> Vec<xylitol_plugin_sdk::resource::SkillInfo> {
        self.agent.loaded_skills()
    }

    pub fn agent_cwd_for_test(&self) -> String {
        self.agent.cwd().to_string()
    }

    fn skill_catalog_pairs(&self) -> Vec<(String, String)> {
        self.agent
            .loaded_skills()
            .iter()
            .map(|s| (s.name.clone(), s.description.clone().unwrap_or_default()))
            .collect()
    }

    pub fn tool_names_for_test(&self) -> Vec<String> {
        self.agent.tool_names()
    }

    pub fn builtins_for_reload(&self) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
        self.agent
            .clone_tools()
            .iter()
            .filter(|t| !xylitol_plugin_sdk::is_mcp_tool_name(t.name()))
            .cloned()
            .collect()
    }

    pub fn system_prompt_for_test(&self) -> Option<String> {
        self.agent.system_prompt().map(String::from)
    }

    pub async fn restore_model(&mut self, model_id: &str) -> Result<(), XyDriverError> {
        self.agent
            .select_model_with_source(model_id, "restore")
            .await
            .map_err(XyDriverError::from)
    }
}

fn bind_session_or_err(
    agent: &mut AgentRuntime,
    session_id: impl Into<String>,
) -> Result<(), XyDriverError> {
    agent.bind_session(session_id).map_err(XyDriverError::from)
}

fn require_active_session(agent: &AgentRuntime) -> Result<&str, XyDriverError> {
    agent
        .session_id()
        .ok_or_else(|| XySessionError::NoActiveSession.into())
}

#[async_trait]
impl XyDriver for XyInProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        self.apply_pending_capability_overlay();
        self.ensure_tool_table_frozen().await;
        if self.agent.session_id().is_none() {
            let id = uuid::Uuid::new_v4().to_string();
            if let Err(e) = bind_session_or_err(&mut self.agent, id.clone()) {
                e.log_failure("driver.run.bind_session");
            }
            self.bind_todo_session(Some(&id)).await;
        } else if let Some(sid) = self.agent.session_id() {
            self.bind_todo_session(Some(sid)).await;
        }
        let stream = self.agent.submit_root(prompt, RunPolicy::Reject).await;
        Box::pin(stream)
    }

    fn abort(&self) {
        self.bang.abort();
        self.agent.abort();
    }

    fn current_model(&self) -> Option<ModelInfo> {
        self.agent.current_model().map(|m| ModelInfo::from(&m))
    }

    fn active_turn(&self) -> Option<(String, String, bool)> {
        let binding = self.agent.inflight_turn_binding()?;
        Some((
            binding.display_name,
            binding.thinking,
            binding.omit_thinking,
        ))
    }

    fn has_active_turn(&self) -> bool {
        self.agent.has_active_turn()
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        self.agent
            .model_registry()
            .list()
            .iter()
            .map(ModelInfo::from)
            .collect()
    }

    fn thinking_level(&self) -> String {
        self.agent.thinking_level()
    }

    fn session_id(&self) -> Option<String> {
        self.agent.session_id().map(String::from)
    }

    fn set_bash_run_sink(&mut self, sink: Option<xylitol_plugin_sdk::BashOutputSink>) {
        *self.bash_run_sink.lock().unwrap_or_else(|e| e.into_inner()) = sink;
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
        XyInProcessDriver::estimate_context_tokens(self).await
    }

    fn capability_snapshot(&self) -> xylitol_plugin_sdk::session::CapabilitySnapshot {
        XyInProcessDriver::capability_snapshot(self)
    }

    fn capability_pending(&self) -> bool {
        XyInProcessDriver::capability_pending(self)
    }

    fn set_capability(
        &mut self,
        id: &str,
        enabled: bool,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        XyInProcessDriver::set_capability(self, id, enabled)
    }

    fn clear_capabilities(
        &mut self,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        XyInProcessDriver::clear_capabilities(self)
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        let mut commands = self.slash_commands.clone();
        for (name, command) in &self.plugin_commands {
            commands.push(CommandInfo {
                name: name.clone(),
                description: command.description().to_string(),
            });
        }
        if let Some(registries) = self
            .ctx
            .as_ref()
            .and_then(|ctx| ctx.get::<xylitol_plugin_sdk::HostRegistries>())
        {
            for (name, command) in registries.plugin_commands.snapshot() {
                if commands.iter().any(|existing| existing.name == name) {
                    continue;
                }
                commands.push(CommandInfo {
                    name,
                    description: command.description().to_string(),
                });
            }
        }
        commands
    }

    fn leaf_entry_id(&self) -> Option<String> {
        XyInProcessDriver::leaf_entry_id(self)
    }

    fn session_store(&self) -> Option<Arc<dyn XySessionStore>> {
        Some(self.store.clone())
    }

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        self.skill_catalog_pairs()
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        self.prepare_loaded_resources().await
    }

    fn mcp_blocks_agent(&self) -> bool {
        self.prepare_hub
            .as_ref()
            .is_some_and(|h| h.view().configured > 0 && !self.prepare_acked)
    }

    async fn begin_session_prepare(&mut self) {
        self.fire_session_prepare(xylitol_plugin_sdk::SessionPrepareReason::Switch);
    }

    async fn poll_prepare_progress(&mut self) -> bool {
        let dirty = self.prepare_poll_dirty();
        let complete = self.prepare_hub.as_ref().is_none_or(|h| h.view().complete);
        if !complete {
            return dirty;
        }
        self.apply_prepare_tools();
        if !self.prepare_acked {
            self.prepare_acked = true;
            return true;
        }
        dirty
    }

    async fn wait_session_prepare(&mut self) {
        let _ = self.join_session_prepare().await;
    }

    async fn reload_runtime(
        &mut self,
        cancel: &CancellationToken,
    ) -> Result<super::types::RuntimeReloadReport, XyDriverError> {
        self.agent.clear_tool_freeze();
        self.fire_session_prepare(xylitol_plugin_sdk::SessionPrepareReason::Reload);
        if let Some(hook) = self.reload_hook.clone() {
            hook.reload(self, cancel).await
        } else {
            Ok(super::types::RuntimeReloadReport::noop())
        }
    }

    fn is_tools_frozen(&self) -> bool {
        self.agent.is_tools_frozen()
    }

    fn take_mcp_gate_notice(&mut self) -> Option<String> {
        self.mcp_gate_notice
            .take()
            .or_else(|| self.prepare_hub.as_ref().and_then(|h| h.take_notice()))
    }

    async fn arm_tool_freeze_gate(&mut self) {
        if self.agent.is_tools_frozen() {
            return;
        }
        self.agent.begin_tool_gating();
        let complete = self.prepare_hub.as_ref().is_none_or(|h| {
            let view = h.view();
            view.configured == 0 || view.complete
        });
        if complete {
            self.apply_prepare_tools();
            let tools = self.agent.clone_tools();
            self.agent.freeze_tools(tools);
            self.prepare_acked = true;
        } else {
            self.fire_session_prepare(xylitol_plugin_sdk::SessionPrepareReason::Switch);
        }
    }

    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<super::proto::BashRun, XyDriverError> {
        let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::user_bash(
            command,
            exclude_from_context,
            self.agent.cwd(),
        );
        self.agent.script_hook_cancel(ty, phase, ctx).await?;

        let bash_id = uuid::Uuid::new_v4().to_string();
        let run = self
            .bash_run_sink
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let cancel = run
            .as_ref()
            .map(|run| run.cancel.clone())
            .unwrap_or_default();
        let (chunk_tx, chunk_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
        let relay =
            run.map(|run| tokio::spawn(relay_bash_chunks(bash_id.clone(), chunk_rx, run.tx)));
        let store = self.store.clone();
        let bang = self.bang.clone();
        let session_id = self.agent.session_id().map(str::to_string);
        let cwd = self.agent.cwd().to_string();
        let command = command.to_string();

        Ok(Box::pin(async move {
            let result = bang
                .execute(
                    store.as_ref(),
                    session_id.as_deref(),
                    &bash_id,
                    &command,
                    exclude_from_context,
                    cancel,
                    Some(chunk_tx),
                    Some(cwd),
                )
                .await;
            if let Some(relay) = relay {
                let _ = relay.await;
            }
            result.map_err(XyDriverError::from)
        }) as super::proto::BashRun)
    }

    async fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        XyInProcessDriver::persist_project_trust(self, mode)
    }
}

impl XyInProcessDriver {
    pub async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        let registry = self.agent.model_registry();
        let found = registry
            .list()
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.id.clone())
            .or_else(|| {
                let hits: Vec<_> = registry
                    .list()
                    .iter()
                    .filter(|m| m.config.model == model_id)
                    .collect();
                match hits.as_slice() {
                    [only] => Some(only.id.clone()),
                    _ => None,
                }
            })
            .ok_or_else(|| XyDriverError::not_found(format!("model not found: {model_id}")))?;
        self.agent
            .select_model(&found)
            .await
            .map_err(XyDriverError::from)?;
        Ok(self
            .agent
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

    pub async fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
        let list = self.agent.model_registry().list().to_vec();
        if list.is_empty() {
            return Err(XyDriverError::not_found("no models available"));
        }
        let current_id = self.agent.current_model().map(|m| m.id.clone());
        let current_idx = current_id
            .as_ref()
            .and_then(|cur| list.iter().position(|m| m.id == *cur))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % list.len();
        let next_id = list[next_idx].id.clone();
        self.agent
            .select_model_with_source(&next_id, "cycle")
            .await
            .map_err(XyDriverError::from)?;
        Ok(ModelInfo::from(&list[next_idx]))
    }

    pub async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        self.agent
            .set_thinking_level(level)
            .await
            .map_err(XyDriverError::from)
    }

    pub fn model_provider(&self, model_id: &str) -> Option<String> {
        self.agent
            .model_registry()
            .list()
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.provider.clone())
    }

    pub async fn compact(&mut self, instructions: Option<String>) -> Result<bool, XyDriverError> {
        self.agent
            .force_compact(instructions)
            .await
            .map(|()| true)
            .map_err(XyDriverError::from)
    }

    pub async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = require_active_session(&self.agent)?.to_string();
        self.exporter
            .export_to_html(self.store.as_ref(), &sid, path)
            .await?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = require_active_session(&self.agent)?.to_string();
        self.exporter
            .export_to_jsonl(self.store.as_ref(), &sid, path)
            .await?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        self.exporter
            .import_from_jsonl(self.store.as_ref(), path)
            .await
    }

    pub async fn steer(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.agent.steer(message);
        Ok(())
    }

    pub async fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.agent.follow_up(message);
        Ok(())
    }

    pub async fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        self.agent.clear_queues(clear_steer, clear_follow_up);
        Ok(())
    }

    pub fn queue_stats(&self) -> crate::agent::QueueStats {
        self.agent.queue_stats()
    }
}

pub async fn relay_bash_chunks(
    bash_id: String,
    mut rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    tx: tokio::sync::mpsc::Sender<xylitol_plugin_sdk::BashChunk>,
) {
    use xylitol_plugin_sdk::BashChunk;

    let mut seq: u64 = 0;
    let mut pending: Vec<u8> = Vec::new();
    while let Some(bytes) = rx.recv().await {
        pending.extend_from_slice(&bytes);
        while !pending.is_empty() {
            let n = utf8_emit_len(&pending);
            if n == 0 {
                break;
            }
            let data = String::from_utf8_lossy(&pending[..n]).into_owned();
            pending.drain(..n);
            seq += 1;
            if tx
                .send(BashChunk {
                    bash_id: bash_id.clone(),
                    seq,
                    data,
                })
                .await
                .is_err()
            {
                return;
            }
        }
    }
    if !pending.is_empty() {
        seq += 1;
        let _ = tx
            .send(BashChunk {
                bash_id,
                seq,
                data: String::from_utf8_lossy(&pending).into_owned(),
            })
            .await;
    }
}

pub fn utf8_emit_len(buf: &[u8]) -> usize {
    match std::str::from_utf8(buf) {
        Ok(_) => buf.len(),
        Err(e) => match e.error_len() {
            None => e.valid_up_to(),
            Some(bad) => e.valid_up_to() + bad,
        },
    }
}
