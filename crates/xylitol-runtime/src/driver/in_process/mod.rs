use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agent::AgentRuntime;
use crate::agent::capabilities::ToolsWriteOutcome;
use crate::driver::bang_exec::BangExecHandler;
use crate::driver::session_export::SessionExporter;
use xylitol_plugin_sdk::AgentTodoGateway;
use xylitol_plugin_sdk::AutomationGateway;
use xylitol_plugin_sdk::WorkflowGateway;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::error::XySessionError;

use super::types::{
    CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, TurnRequest,
};
pub(super) use super::{XyDriver, XyDriverError, types};
use xylitol_plugin_sdk::message::AgentMessage;

mod actor;
mod prepare;
mod reload;
mod session;

pub use prepare::InProcessReloadHook;
pub use reload::{ReloadContext, ReloadParticipant, ReloadRegistry};
pub use session::XySessionOps;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum OverlayPhase {
    #[default]
    Idle,
    Pending,
}

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
    actor: Option<actor::ActorState>,
    slot: Option<Arc<actor::RuntimeSlot>>,
    actor_task: Option<tokio::task::JoinHandle<()>>,
    handle: actor::DriverHandle,
    pub store: Arc<dyn XySessionStore>,
    bang: BangExecHandler,
    exporter: SessionExporter,
    ask_gateway: Option<Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>>,
    todo_gateway: Arc<dyn AgentTodoGateway>,
    automation_gateway: Option<Arc<dyn AutomationGateway>>,
    workflow_gateway: Option<Arc<dyn WorkflowGateway>>,
    bash_run_sink: std::sync::Mutex<Option<xylitol_plugin_sdk::BashOutputSink>>,
    slash_commands: Vec<CommandInfo>,
    ctx: Option<xylitol_plugin_sdk::rt::Ctx>,
    prepare_hub: Option<Arc<dyn xylitol_plugin_sdk::SessionPrepareHub>>,
    reload_hook: Option<Arc<dyn InProcessReloadHook>>,
    reload_cwd: Option<std::path::PathBuf>,
    reload_agent_dir: Option<std::path::PathBuf>,
    reload_config_path: Option<std::path::PathBuf>,
    trust_audit: crate::trust::TrustAuditConfig,
    reload_registry: Arc<ReloadRegistry>,
    last_ui_mcp_label: Option<Option<String>>,
    mcp_gate_notice: Option<String>,
    session_profile: xylitol_plugin_sdk::session::SessionProfile,
    profile_mismatch_notice: Option<String>,
    prepare_acked: bool,
    tool_gate_poll_started: Option<std::time::Instant>,
    base_tools: Option<crate::agent::tools::ToolSet>,
    capability_disabled: BTreeSet<String>,
    capability_applied: BTreeSet<String>,
    overlay_phase: OverlayPhase,
    mcp_server_ids: Vec<String>,
    plugin_commands: Vec<(String, Arc<dyn xylitol_plugin_sdk::XyPluginCommand>)>,
}

impl XyInProcessDriver {
    /// ~keep 跨 crate 接缝：kernel 的 subagent 与 occupancy 需要直读 runtime 状态。
    pub fn state(&mut self) -> &actor::ActorState {
        self.state_mut()
    }

    /// ~keep 跨 crate 接缝：kernel 的 subagent 需要独占写 runtime。
    pub fn state_mut(&mut self) -> &mut actor::ActorState {
        if self.actor.is_none()
            && let Some(slot) = self.slot.as_ref()
            && let Some(state) = actor::ActorState::from_slot(slot, self.handle.shared().clone())
        {
            self.actor = Some(state);
        }
        self.actor
            .as_mut()
            .expect("driver runtime unavailable: actor holds it")
    }

    pub(super) async fn dispatch_script_hook_cancel(
        &mut self,
        event_type: &str,
        phase: &str,
        context: serde_json::Value,
    ) -> Result<(), XyDriverError> {
        self.state_mut()
            .runtime()
            .script_hook_cancel(event_type, phase, context)
            .await
            .map_err(XyDriverError::from)
    }

    pub(super) async fn dispatch_script_hook_observe(
        &mut self,
        event_type: &str,
        phase: &str,
        context: serde_json::Value,
    ) {
        self.state_mut()
            .runtime()
            .script_hook_observe(event_type, phase, context)
            .await;
    }

    async fn ensure_runtime_session(&mut self, id: &str) -> Result<(), XyDriverError> {
        self.state_mut()
            .runtime()
            .ensure_session(id, None)
            .await
            .map_err(XyDriverError::from)
    }

    fn snapshot(&self) -> actor::DriverSnapshot {
        match self.actor.as_ref() {
            Some(state) => {
                actor::DriverSnapshot::from_runtime(state.runtime(), state.capability.clone())
            }
            None => {
                let mut snapshot = self.handle.snapshot();
                if let Some(session_id) = self.handle.shared().session_id() {
                    snapshot.session_id = Some(session_id);
                }
                snapshot
            }
        }
    }

    fn ensure_actor_spawned(&mut self) {
        if self.handle.is_spawned() || self.actor.is_none() {
            return;
        }
        let Some(state) = self.actor.take() else {
            return;
        };
        let shared = state.shared.clone();
        let slot = actor::RuntimeSlot::new(state.into_runtime());
        let (handle, task) = actor::DriverHandle::spawn_parked(&slot, shared);
        self.slot = Some(slot);
        self.actor_task = Some(task);
        self.handle = handle;
    }

    fn try_apply_prepare_tools(&mut self) -> bool {
        if self.actor.is_none()
            && let Some(slot) = self.slot.as_ref()
            && let Some(state) = actor::ActorState::from_slot(slot, self.handle.shared().clone())
        {
            self.actor = Some(state);
        }
        if self.actor.is_none() {
            return false;
        }
        self.apply_prepare_tools();
        true
    }

    pub async fn bind_session(&mut self, session_id: String) -> Result<(), XyDriverError> {
        self.ensure_actor_spawned();
        self.handle
            .request(|reply| actor::DriverCommand::BindSession { session_id, reply })
            .await?
    }

    pub fn model_registry(&mut self) -> crate::agent::model::registry::ModelRegistry {
        self.state_mut().runtime_mut().model_registry()
    }

    pub fn replace_model_registry(
        &mut self,
        registry: crate::agent::model::registry::ModelRegistry,
    ) {
        self.state_mut()
            .runtime_mut()
            .replace_model_registry(registry);
    }

    pub async fn join_actor(&mut self) {
        if let Some(task) = self.actor_task.take() {
            let _ = task.await;
        }
    }

    pub fn current_model_id(&self) -> Option<String> {
        self.snapshot().current_model.map(|meta| meta.id)
    }

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
        let commands = Arc::new(Vec::new());
        let state = actor::ActorState::new(agent, commands);
        let handle = actor::DriverHandle::detached(&state);
        Self {
            actor: Some(state),
            slot: None,
            actor_task: None,
            handle,
            store,
            bang,
            exporter,
            ask_gateway: None,
            todo_gateway: Arc::new(NoopTodo),
            automation_gateway: None,
            workflow_gateway: None,
            bash_run_sink: std::sync::Mutex::new(None),
            slash_commands: Vec::new(),
            ctx: None,
            prepare_hub: None,
            reload_hook: None,
            reload_cwd: None,
            reload_agent_dir: None,
            reload_config_path: None,
            trust_audit: crate::trust::TrustAuditConfig::default(),
            last_ui_mcp_label: None,
            mcp_gate_notice: None,
            session_profile: xylitol_plugin_sdk::session::SessionProfile::default(),
            profile_mismatch_notice: None,
            prepare_acked: true,
            tool_gate_poll_started: None,
            base_tools: None,
            capability_disabled: BTreeSet::new(),
            capability_applied: BTreeSet::new(),
            overlay_phase: OverlayPhase::Idle,
            mcp_server_ids: Vec::new(),
            plugin_commands: Vec::new(),
            reload_registry: Arc::new(ReloadRegistry::new()),
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
        self.handle.shared().set_todo_gateway(Arc::clone(&todo));
        self.todo_gateway = todo;
    }

    pub fn set_automation_gateway(&mut self, automation: Arc<dyn AutomationGateway>) {
        self.handle
            .shared()
            .set_automation_gateway(Arc::clone(&automation));
        self.automation_gateway = Some(automation);
    }

    pub fn set_workflow_gateway(&mut self, workflow: Arc<dyn WorkflowGateway>) {
        self.handle
            .shared()
            .set_workflow_gateway(Arc::clone(&workflow));
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
        self.plugin_commands
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, command)| Arc::clone(command))
            .or_else(|| self.ctx_plugin_command(name))
    }

    fn ctx_plugin_command(
        &self,
        name: &str,
    ) -> Option<Arc<dyn xylitol_plugin_sdk::XyPluginCommand>> {
        let registries = self
            .ctx
            .as_ref()?
            .get::<xylitol_plugin_sdk::HostRegistries>()?;
        registries.plugin_commands.get(name)
    }

    fn plugin_command_infos(&self) -> Vec<CommandInfo> {
        let mut commands: Vec<CommandInfo> = self
            .plugin_commands
            .iter()
            .map(|(name, command)| CommandInfo {
                name: name.clone(),
                description: command.description().to_string(),
            })
            .collect();
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
        if let Some(automation) = self.automation_gateway.as_ref() {
            automation.bind_session(session_id.clone()).await;
        }
        self.bind_optional_gateways(session_id).await;
    }

    async fn bind_optional_gateways(&self, session_id: Option<String>) {
        if let Some(workflow) = self.workflow_gateway.as_ref() {
            workflow.bind_session(session_id).await;
        }
    }

    pub fn todo_gateway(&self) -> Arc<dyn AgentTodoGateway> {
        self.todo_gateway.clone()
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.handle.shared().cancel_token()
    }

    pub fn set_tools(&mut self, tools: crate::agent::tools::ToolSet) -> ToolsWriteOutcome {
        if self.actor.is_none()
            && let Some(slot) = self.slot.as_ref()
            && let Some(state) = actor::ActorState::from_slot(slot, self.handle.shared().clone())
        {
            self.actor = Some(state);
        }
        match self.actor.as_mut() {
            Some(state) => state.runtime_mut().set_tools(tools),
            None => {
                self.handle.shared().set_pending_tools(tools);
                ToolsWriteOutcome::Applied
            }
        }
    }

    pub fn set_base_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.base_tools = Some(tools);
    }

    pub fn set_mcp_server_ids(&mut self, ids: Vec<String>) {
        self.mcp_server_ids = ids;
    }

    pub fn capability_snapshot(&self) -> xylitol_plugin_sdk::session::CapabilitySnapshot {
        let base = self.base_tools.clone().unwrap_or_default();
        crate::agent::tools::capability_snapshot(
            &base,
            &self.mcp_server_ids,
            &self.capability_disabled,
            &self.capability_applied,
        )
    }

    pub fn capability_pending(&self) -> bool {
        debug_assert!(
            self.overlay_phase != OverlayPhase::Idle
                || self.capability_disabled == self.capability_applied,
            "capability overlay phase out of sync with applied set"
        );
        self.overlay_phase == OverlayPhase::Pending
    }

    pub fn install_base_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.base_tools = Some(tools.clone());
        let filtered = crate::agent::tools::filter_tools_by_capability_overlay(
            tools,
            &self.capability_disabled,
        );
        self.state_mut().runtime_mut().reopen_tools_for_regate();
        self.state_mut().runtime_mut().freeze_tools(filtered);
        self.capability_applied = self.capability_disabled.clone();
        self.overlay_phase = OverlayPhase::Idle;
        debug_assert!(
            self.snapshot().tools_frozen,
            "install_base_tools must end Frozen"
        );
    }

    pub fn apply_pending_capability_overlay(&mut self) {
        if !self.capability_pending() {
            return;
        }
        let base = self.base_tools.clone().unwrap_or_default();
        let filtered = crate::agent::tools::filter_tools_by_capability_overlay(
            base,
            &self.capability_disabled,
        );
        self.state_mut().runtime_mut().reopen_tools_for_regate();
        match self.state_mut().runtime_mut().set_tools(filtered.clone()) {
            ToolsWriteOutcome::Applied => self.state_mut().runtime_mut().freeze_tools(filtered),
            ToolsWriteOutcome::RejectedMidTurn | ToolsWriteOutcome::RejectedFrozen => {
                let current = self.state().runtime().clone_tools();
                self.state_mut().runtime_mut().freeze_tools(current);
            }
        }
        self.capability_applied = self.capability_disabled.clone();
        self.overlay_phase = OverlayPhase::Idle;
        debug_assert!(
            self.snapshot().tools_frozen,
            "capability overlay must end Frozen"
        );
    }

    pub fn freeze_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.state_mut().runtime_mut().freeze_tools(tools);
    }

    pub fn reopen_tools_for_regate(&mut self) {
        self.state_mut().runtime_mut().reopen_tools_for_regate();
    }

    pub fn is_tools_frozen(&self) -> bool {
        self.snapshot().tools_frozen
    }

    pub async fn ensure_tool_table_frozen(&mut self) {
        if self.snapshot().tools_frozen {
            return;
        }
        debug_assert!(
            self.state().runtime().tool_freeze_phase()
                != crate::agent::tools::ToolFreezePhase::Gating,
            "tool table left in Gating outside an assembly line"
        );
        self.state_mut().runtime_mut().begin_tool_gating();
        let _ = self.join_session_prepare().await;
        let tools = self.state().runtime().clone_tools();
        self.state_mut().runtime_mut().freeze_tools(tools);
        self.prepare_acked = true;
        self.handle.shared().set_prepare_acked(true);
    }

    pub fn set_workspaces(&mut self, workspaces: Vec<String>) {
        if self.actor.is_none()
            && let Some(slot) = self.slot.as_ref()
            && let Some(state) = actor::ActorState::from_slot(slot, self.handle.shared().clone())
        {
            self.actor = Some(state);
        }
        match self.actor.as_mut() {
            Some(state) => state.runtime_mut().set_workspaces(workspaces),
            None => self.handle.shared().set_pending_workspaces(workspaces),
        }
    }

    pub fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        let cwd = self
            .reload_cwd
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| self.snapshot().cwd.as_str().to_string());
        let mgr = crate::trust::TrustManager::with_audit(
            self.reload_agent_dir
                .clone()
                .unwrap_or_else(crate::trust::TrustManager::default_dir),
            self.trust_audit.clone(),
        );
        let options = mgr.get_trust_options(&cwd, false);
        let Some(opt) = crate::trust::select_option(&options, trust_mode_key(mode)) else {
            return Err(match mode {
                ProjectTrustMode::TrustParent => {
                    XyDriverError::invalid_input("no parent folder to trust")
                }
                _ => XyDriverError::invalid_input("trust option unavailable"),
            });
        };
        let audit = crate::trust::TrustAuditContext::new(
            crate::trust::TrustActor::Host,
            crate::trust::TrustSource::HostUnary,
        );
        let applied = mgr.apply_option(opt, &cwd, &audit)?;
        let saved = applied.saved_path.unwrap_or_else(|| cwd.clone());
        let verb = if applied.trusted { "trusted" } else { "denied" };
        Ok(ProjectTrustPersistReport {
            trusted: applied.trusted,
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
        self.state_mut().runtime_mut().apply_prompt_resources(
            context_files,
            system_prompt,
            append_system_prompt,
        );
    }

    pub fn apply_skills(&mut self, skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>) {
        self.state_mut().runtime_mut().apply_skills(skills);
    }

    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.snapshot().skill_names
    }

    pub fn loaded_skills(&self) -> Vec<xylitol_plugin_sdk::resource::SkillInfo> {
        Vec::new()
    }

    /// ~keep 测试接缝：仅供外部集成测试观测 agent 工作目录，非产品 API。
    /// ~keep 调用方：`xylitol-kernel` occupancy factory 转发与 `tests/occupancy/assembly_fixture_tests.rs`。
    /// ~keep 不得挂 `#[cfg(test)]`：调用方跨 crate，cfg 只在被测 crate 自身测试编译时激活。
    pub fn agent_cwd_for_test(&self) -> String {
        self.snapshot().cwd.as_str().to_string()
    }

    fn skill_catalog_pairs(&self) -> Vec<(String, String)> {
        self.snapshot()
            .skill_names
            .into_iter()
            .map(|name| (name, String::new()))
            .collect()
    }

    /// ~keep 测试接缝：仅供外部测试断言已注册工具表，非产品 API。
    /// ~keep 调用方：`crates/xylitol-kernel/src/composition.rs`。
    /// ~keep 不得挂 `#[cfg(test)]`：调用方跨 crate，cfg 只在被测 crate 自身测试编译时激活。
    pub fn tool_names_for_test(&self) -> Vec<String> {
        self.snapshot().tool_names
    }

    pub fn builtins_for_reload(&mut self) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
        self.state_mut()
            .runtime_mut()
            .clone_tools()
            .iter()
            .filter(|t| !xylitol_plugin_sdk::is_mcp_tool_name(t.name()))
            .cloned()
            .collect()
    }

    /// ~keep 测试接缝：仅供外部测试断言系统提示词装配，非产品 API。
    /// ~keep 调用方：`crates/xylitol-kernel/src/bootstrap_tests.rs`。
    /// ~keep 不得挂 `#[cfg(test)]`：调用方跨 crate，cfg 只在被测 crate 自身测试编译时激活。
    pub fn system_prompt_for_test(&self) -> Option<String> {
        self.snapshot().system_prompt
    }

    pub async fn restore_model(&mut self, model_id: &str) -> Result<(), XyDriverError> {
        self.state_mut()
            .runtime_mut()
            .select_model_with_source(model_id, "restore")
            .await
            .map_err(XyDriverError::from)
    }
}

fn trust_mode_key(mode: ProjectTrustMode) -> &'static str {
    match mode {
        ProjectTrustMode::TrustCwd => "trust",
        ProjectTrustMode::TrustParent => "trust_parent",
        ProjectTrustMode::Deny => "deny",
    }
}

pub(super) fn bind_session_or_err(
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
    async fn steer(&mut self, message: &str) -> Result<String, XyDriverError> {
        Ok(self.handle.shared().steer(message))
    }

    async fn follow_up(&mut self, message: &str) -> Result<String, XyDriverError> {
        Ok(self.handle.shared().follow_up(message))
    }

    fn queue_items(&self) -> Vec<xylitol_plugin_sdk::session::QueueItem> {
        self.handle.shared().queue_items()
    }

    fn revoke_queue_item(&self, id: &str) -> xylitol_plugin_sdk::RevokeQueueItemOutcome {
        let outcome = self.handle.shared().revoke_queue_item(id);
        xylitol_plugin_sdk::RevokeQueueItemOutcome {
            revoked: outcome.revoked,
            items: outcome.items,
        }
    }

    async fn run(&mut self, prompt: &str) -> EventStream {
        self.run_with_context(prompt, Vec::new()).await
    }

    async fn run_with_context(
        &mut self,
        prompt: &str,
        contexts: Vec<xylitol_plugin_sdk::mcp::McpResourceContext>,
    ) -> EventStream {
        self.prepare_run().await;
        if let Some(sid) = self.snapshot().session_id.as_deref() {
            for context in contexts {
                let entry = xylitol_plugin_sdk::session::SessionEntry::CustomMessage(
                    xylitol_plugin_sdk::session::CustomMessageEntry {
                        base: xylitol_plugin_sdk::session::EntryBase {
                            entry_type: "customMessage".into(),
                            id: String::new(),
                            parent_id: None,
                            timestamp: 0,
                        },
                        custom_type: "mcpResource".into(),
                        content: serde_json::Value::String(context.text),
                        display: true,
                        details: Some(serde_json::json!({
                            "server": context.server,
                            "uri": context.uri,
                            "mime": context.mime,
                            "truncated": context.truncated,
                        })),
                    },
                );
                if let Err(error) = self.store.append_session_entry(sid, &entry).await {
                    log::warn!(
                        target: "xylitol::runtime",
                        "append mcpResource context failed session={sid} error={error}"
                    );
                }
            }
        }
        if !self.handle.shared().phase().is_idle() {
            return Box::pin(crate::agent::runtime::XyEventStream::busy());
        }
        self.ensure_actor_spawned();
        self.handle
            .run(
                vec![xylitol_plugin_sdk::message::AgentPart::text(prompt)],
                Vec::new(),
                None,
            )
            .await
    }

    async fn run_turn(&mut self, request: TurnRequest) -> Result<EventStream, XyDriverError> {
        self.prepare_run().await;
        let context = request.context.into_iter().map(AgentMessage::Llm).collect();
        self.ensure_actor_spawned();
        let stream = self
            .handle
            .run(request.parts, context, request.system_prompt)
            .await;
        Ok(Box::pin(stream))
    }

    async fn mcp_catalog(&mut self) -> Result<xylitol_plugin_sdk::mcp::McpCatalog, XyDriverError> {
        let Some(hub) = self.prepare_hub.as_ref() else {
            return Ok(xylitol_plugin_sdk::mcp::McpCatalog::default());
        };
        let sid = self.snapshot().session_id.unwrap_or_default();
        Ok(hub.mcp_catalog_for_session(&sid).await)
    }

    async fn mcp_resource_read(
        &mut self,
        server: &str,
        uri: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpResourceContent, XyDriverError> {
        let Some(hub) = self.prepare_hub.as_ref() else {
            return Err(XyDriverError::unsupported("mcp_resource_read"));
        };
        let sid = self.snapshot().session_id.unwrap_or_default();
        hub.mcp_resource_read(&sid, server, uri)
            .await
            .map_err(XyDriverError::from)
    }

    async fn mcp_prompt_get(
        &mut self,
        server: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, XyDriverError> {
        let Some(hub) = self.prepare_hub.as_ref() else {
            return Err(XyDriverError::unsupported("mcp_prompt_get"));
        };
        let sid = self.snapshot().session_id.unwrap_or_default();
        hub.mcp_prompt_get(&sid, server, name, arguments)
            .await
            .map_err(XyDriverError::from)
    }

    async fn mcp_oauth_start(
        &mut self,
        server: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpOAuthFlow, XyDriverError> {
        let Some(hub) = self.prepare_hub.as_ref() else {
            return Err(XyDriverError::unsupported("mcp_oauth_start"));
        };
        hub.mcp_oauth_start(server)
            .await
            .map_err(XyDriverError::remote)
    }

    async fn mcp_oauth_cancel(&mut self, flow_id: &str) -> Result<(), XyDriverError> {
        let Some(hub) = self.prepare_hub.as_ref() else {
            return Err(XyDriverError::unsupported("mcp_oauth_cancel"));
        };
        hub.mcp_oauth_cancel(flow_id)
            .await
            .map_err(XyDriverError::remote)
    }

    async fn mcp_oauth_status(
        &mut self,
        flow_id: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpOAuthStatus, XyDriverError> {
        let Some(hub) = self.prepare_hub.as_ref() else {
            return Err(XyDriverError::unsupported("mcp_oauth_status"));
        };
        Ok(hub.mcp_oauth_status(flow_id).await)
    }

    fn abort(&self) {
        self.bang.abort();
        self.handle.shared().abort();
    }

    fn current_model(&self) -> Option<ModelInfo> {
        self.snapshot().current_model.as_ref().map(ModelInfo::from)
    }

    fn active_turn(&self) -> Option<(String, String, bool)> {
        let binding = self.snapshot().active_turn?;
        Some((binding.0, binding.1, binding.2))
    }

    fn has_active_turn(&self) -> bool {
        self.snapshot().has_active_turn
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        self.snapshot()
            .model_list
            .iter()
            .map(ModelInfo::from)
            .collect()
    }

    fn thinking_level(&self) -> String {
        self.snapshot().thinking_level
    }

    fn session_id(&self) -> Option<String> {
        self.snapshot().session_id.as_deref().map(String::from)
    }

    fn set_bash_run_sink(&mut self, sink: Option<xylitol_plugin_sdk::BashOutputSink>) {
        *self.bash_run_sink.lock().unwrap_or_else(|e| e.into_inner()) = sink;
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
        XySessionOps::estimate_context_tokens(self).await
    }

    fn capability_snapshot(&self) -> xylitol_plugin_sdk::session::CapabilitySnapshot {
        XyInProcessDriver::capability_snapshot(self)
    }

    fn capability_pending(&self) -> bool {
        XyInProcessDriver::capability_pending(self)
    }

    async fn set_capability(
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
        self.overlay_phase = OverlayPhase::Pending;
        Ok(self.capability_snapshot())
    }

    async fn clear_capabilities(
        &mut self,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        self.capability_disabled.clear();
        self.overlay_phase = OverlayPhase::Pending;
        Ok(self.capability_snapshot())
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        let mut commands = self.slash_commands.clone();
        commands.extend(self.plugin_command_infos());
        commands
    }

    fn leaf_entry_id(&self) -> Option<String> {
        XySessionOps::leaf_entry_id(self)
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
            .is_some_and(|h| h.view().configured > 0 && !self.handle.shared().prepare_acked())
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
        if self.try_apply_prepare_tools() && !self.handle.shared().prepare_acked() {
            self.prepare_acked = true;
            self.handle.shared().set_prepare_acked(true);
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
        self.state_mut().runtime_mut().clear_tool_freeze();
        self.fire_session_prepare(xylitol_plugin_sdk::SessionPrepareReason::Reload);
        if let Some(hook) = self.reload_hook.clone() {
            hook.reload(self, cancel).await
        } else {
            Ok(super::types::RuntimeReloadReport::noop())
        }
    }

    fn is_tools_frozen(&self) -> bool {
        self.snapshot().tools_frozen
    }

    async fn take_mcp_gate_notice(&mut self) -> Option<String> {
        self.mcp_gate_notice
            .take()
            .or_else(|| self.prepare_hub.as_ref().and_then(|h| h.take_notice()))
    }

    async fn arm_tool_freeze_gate(&mut self) {
        if self.snapshot().tools_frozen {
            return;
        }
        self.state_mut().runtime_mut().begin_tool_gating();
        let complete = self.prepare_hub.as_ref().is_none_or(|h| {
            let view = h.view();
            view.configured == 0 || view.complete
        });
        if complete {
            self.apply_prepare_tools();
            let tools = self.state().runtime().clone_tools();
            self.state_mut().runtime_mut().freeze_tools(tools);
            self.prepare_acked = true;
            self.handle.shared().set_prepare_acked(true);
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
            self.snapshot().cwd.as_str(),
        );
        self.state_mut()
            .runtime()
            .script_hook_cancel(ty, phase, ctx)
            .await?;

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
        let session_id = self.snapshot().session_id.as_deref().map(str::to_string);
        let cwd = self.snapshot().cwd.as_str().to_string();
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
    async fn prepare_run(&mut self) {
        self.apply_pending_capability_overlay();
        self.ensure_tool_table_frozen().await;
        if self.snapshot().session_id.as_deref().is_none() {
            let id = uuid::Uuid::new_v4().to_string();
            if let Err(e) = bind_session_or_err(self.state_mut().runtime_mut(), id.clone()) {
                e.log_failure("driver.run.bind_session");
            }
            self.bind_todo_session(Some(&id)).await;
        } else if let Some(sid) = self.snapshot().session_id {
            self.bind_todo_session(Some(&sid)).await;
        }
    }

    pub async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        self.ensure_actor_spawned();
        if self.handle.is_spawned() {
            return self
                .handle
                .request(|reply| actor::DriverCommand::SelectModel {
                    model_id: model_id.to_string(),
                    reply,
                })
                .await?;
        }
        let registry = self.state().runtime().model_registry();
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
        self.state_mut()
            .runtime_mut()
            .select_model(&found)
            .await
            .map_err(XyDriverError::from)?;
        Ok(self
            .state_mut()
            .runtime_mut()
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
        self.ensure_actor_spawned();
        if self.handle.is_spawned() {
            return self
                .handle
                .request(|reply| actor::DriverCommand::CycleModel { reply })
                .await?;
        }
        let list = self.state().runtime().model_registry().list().to_vec();
        if list.is_empty() {
            return Err(XyDriverError::not_found("no models available"));
        }
        let current_id = self.state().runtime().current_model().map(|m| m.id.clone());
        let current_idx = current_id
            .as_ref()
            .and_then(|cur| list.iter().position(|m| m.id == *cur))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % list.len();
        let next_id = list[next_idx].id.clone();
        self.state_mut()
            .runtime_mut()
            .select_model_with_source(&next_id, "cycle")
            .await
            .map_err(XyDriverError::from)?;
        Ok(ModelInfo::from(&list[next_idx]))
    }

    pub async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        self.ensure_actor_spawned();
        if self.handle.is_spawned() {
            return self
                .handle
                .request(|reply| actor::DriverCommand::SetThinkingLevel { level, reply })
                .await?;
        }
        self.state_mut()
            .runtime_mut()
            .set_thinking_level(level)
            .await
            .map_err(XyDriverError::from)
    }

    pub fn model_provider(&self, model_id: &str) -> Option<String> {
        self.snapshot()
            .model_list
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.provider.clone())
    }

    pub async fn compact(&mut self, instructions: Option<String>) -> Result<bool, XyDriverError> {
        self.ensure_actor_spawned();
        if self.handle.is_spawned() {
            return self
                .handle
                .request(|reply| actor::DriverCommand::Compact {
                    instructions,
                    reply,
                })
                .await?;
        }
        self.state_mut()
            .runtime_mut()
            .force_compact(instructions)
            .await
            .map(|()| true)
            .map_err(XyDriverError::from)
    }

    pub async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = require_active_session(self.state().runtime())?.to_string();
        self.exporter
            .export_to_html(self.store.as_ref(), &sid, path)
            .await?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = require_active_session(self.state().runtime())?.to_string();
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

    pub async fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        self.state()
            .runtime()
            .clear_queues(clear_steer, clear_follow_up);
        Ok(())
    }

    pub fn queue_stats(&self) -> crate::agent::QueueStats {
        self.handle.shared().queue_stats()
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

#[cfg(test)]
mod persist_trust_tests {
    use std::sync::Arc;

    use xylitol_plugin_sdk::XySessionStore;

    use crate::agent::runtime::baseline::RuntimeBaseline;
    use crate::driver::in_process::XyInProcessDriver;
    use crate::driver::types::ProjectTrustMode;
    use crate::test_support::{MemoryStore, NoopSink};

    use super::trust_mode_key;

    fn trust_driver(cwd: &str, store: Arc<dyn XySessionStore>) -> XyInProcessDriver {
        let baseline = RuntimeBaseline {
            store: store.clone(),
            sink: Arc::new(NoopSink),
            permission: crate::permission::allow_all_permission(),
            hook_bus: None,
            model_registry: crate::agent::ModelRegistry::new(),
            model_builder: crate::test_support::dummy_model_builder(),
            cwd: cwd.to_string(),
            steering_mode: crate::agent::capabilities::QueueMode::default(),
            follow_up_mode: crate::agent::capabilities::QueueMode::default(),
            compaction_settings: None,
            compaction_factory: xylitol_plugin_sdk::noop_compaction_factory(),
            tools: crate::agent::tools::ToolSet::empty(),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            skills: Vec::new(),
            batch_mode: xylitol_plugin_sdk::XyBatchMode::Sequential,
        };
        XyInProcessDriver::new(baseline.materialize_runtime(), store)
    }

    fn canonical(path: &std::path::Path) -> String {
        std::fs::canonicalize(path)
            .expect("canonicalize")
            .to_string_lossy()
            .into_owned()
    }

    #[tokio::test]
    async fn trust_cwd_without_reload_state_uses_agent_cwd() {
        let project = tempfile::tempdir().expect("project dir");
        let agent_dir = tempfile::tempdir().expect("agent dir");
        let cwd = project.path().to_string_lossy().into_owned();
        let mut driver = trust_driver(&cwd, Arc::new(MemoryStore::new()));
        driver.reload_agent_dir = Some(agent_dir.path().to_path_buf());
        let report = driver
            .persist_project_trust(ProjectTrustMode::TrustCwd)
            .expect("trust cwd must persist");
        assert!(report.trusted);
        let expected = canonical(project.path());
        assert_eq!(report.saved_path.as_deref(), Some(expected.as_str()));
        let mgr = crate::trust::TrustManager::new(agent_dir.path().to_path_buf());
        assert_eq!(mgr.is_project_trusted(&expected), Some(true));
    }

    #[tokio::test]
    async fn trust_parent_persists_parent_decision() {
        let root = tempfile::tempdir().expect("root dir");
        let agent_dir = tempfile::tempdir().expect("agent dir");
        let project = root.path().join("proj");
        std::fs::create_dir_all(&project).expect("create project");
        let cwd = project.to_string_lossy().into_owned();
        let mut driver = trust_driver(&cwd, Arc::new(MemoryStore::new()));
        driver.enable_reload_state(project.clone(), agent_dir.path().to_path_buf(), false);
        let report = driver
            .persist_project_trust(ProjectTrustMode::TrustParent)
            .expect("trust parent must persist");
        assert!(report.trusted);
        let expected_root = canonical(root.path());
        assert_eq!(report.saved_path.as_deref(), Some(expected_root.as_str()));
        let mgr = crate::trust::TrustManager::new(agent_dir.path().to_path_buf());
        assert_eq!(mgr.is_project_trusted(&expected_root), Some(true));
        assert_eq!(mgr.is_project_trusted(&canonical(&project)), Some(true));
    }

    #[tokio::test]
    async fn trust_parent_at_filesystem_root_errors() {
        let agent_dir = tempfile::tempdir().expect("agent dir");
        let mut driver = trust_driver("/", Arc::new(MemoryStore::new()));
        driver.enable_reload_state(
            std::path::PathBuf::from("/"),
            agent_dir.path().to_path_buf(),
            false,
        );
        let err = driver
            .persist_project_trust(ProjectTrustMode::TrustParent)
            .expect_err("filesystem root has no parent to trust");
        let text = err.to_string();
        assert!(text.contains("no parent folder to trust"), "{text}");
    }

    #[tokio::test]
    async fn deny_persists_untrusted_decision() {
        let project = tempfile::tempdir().expect("project dir");
        let agent_dir = tempfile::tempdir().expect("agent dir");
        let cwd = project.path().to_string_lossy().into_owned();
        let mut driver = trust_driver(&cwd, Arc::new(MemoryStore::new()));
        driver.enable_reload_state(
            project.path().to_path_buf(),
            agent_dir.path().to_path_buf(),
            false,
        );
        let report = driver
            .persist_project_trust(ProjectTrustMode::Deny)
            .expect("deny must persist");
        assert!(!report.trusted);
        let expected = canonical(project.path());
        assert_eq!(report.saved_path.as_deref(), Some(expected.as_str()));
        let mgr = crate::trust::TrustManager::new(agent_dir.path().to_path_buf());
        assert_eq!(mgr.is_project_trusted(&expected), Some(false));
    }

    #[tokio::test]
    async fn persist_project_trust_selects_by_key_after_label_tampering() {
        let root = tempfile::tempdir().expect("root dir");
        let agent_dir = tempfile::tempdir().expect("agent dir");
        let project = root.path().join("proj");
        std::fs::create_dir_all(&project).expect("create project");
        let cwd = project.to_string_lossy().into_owned();

        let mgr = crate::trust::TrustManager::new(agent_dir.path().to_path_buf());
        let mut options = mgr.get_trust_options(&cwd, false);
        for opt in &mut options {
            opt.label = "Do not trust".into();
        }
        for (mode, expected_key) in [
            (ProjectTrustMode::TrustCwd, "trust"),
            (ProjectTrustMode::TrustParent, "trust_parent"),
            (ProjectTrustMode::Deny, "deny"),
        ] {
            let selected = crate::trust::select_option(&options, trust_mode_key(mode))
                .unwrap_or_else(|| panic!("{expected_key} must be selected by key"));
            assert_eq!(selected.key, expected_key);
        }

        let mut driver = trust_driver(&cwd, Arc::new(MemoryStore::new()));
        driver.enable_reload_state(project.clone(), agent_dir.path().to_path_buf(), false);
        let report = driver
            .persist_project_trust(ProjectTrustMode::Deny)
            .expect("deny must persist after label tampering");
        assert!(!report.trusted);
        let store_mgr = crate::trust::TrustManager::new(agent_dir.path().to_path_buf());
        assert_eq!(
            store_mgr.is_project_trusted(&canonical(&project)),
            Some(false)
        );
    }
}
