use std::sync::Arc;

use tokio::sync::mpsc;

use crate::agent::AgentRuntime;
use crate::agent::capabilities::{AsyncQueueRuntime, QueueStats, RevokeOutcome};
use crate::agent::runtime::state::RuntimePhase;
use crate::agent::runtime::{XyEventStream, state};

use super::XyDriverError;
use super::types::{CommandInfo, EventStream, ModelInfo};

pub const MAILBOX_CAPACITY: usize = 64;

#[derive(Debug, Clone, Default)]
pub struct DriverSnapshot {
    pub session_id: Option<String>,
    pub cwd: String,
    pub workspaces: Vec<String>,
    pub thinking_level: String,
    pub current_model: Option<xylitol_plugin_sdk::model::XyModelMeta>,
    pub tools_frozen: bool,
    pub tool_names: Vec<String>,
    pub skill_names: Vec<String>,
    pub loop_middleware_ids: Vec<String>,
    pub active_turn: Option<(String, String, bool)>,
    pub has_active_turn: bool,
    pub fixed_context: crate::agent::compaction::FixedRequestContext,
    pub system_prompt: Option<String>,
    pub model_list: Vec<xylitol_plugin_sdk::model::XyModelMeta>,
    pub capability_pending: bool,
    pub capability_snapshot: xylitol_plugin_sdk::session::CapabilitySnapshot,
}

impl DriverSnapshot {
    pub fn from_runtime(runtime: &AgentRuntime, capability: CapabilityView) -> Self {
        Self {
            session_id: runtime.session_id().map(str::to_string),
            cwd: runtime.cwd().to_string(),
            workspaces: runtime.workspaces().to_vec(),
            thinking_level: runtime.thinking_level(),
            current_model: runtime.current_model(),
            tools_frozen: runtime.is_tools_frozen(),
            tool_names: runtime.tool_names(),
            skill_names: runtime
                .loaded_skills()
                .iter()
                .map(|skill| skill.name.clone())
                .collect(),
            loop_middleware_ids: runtime.loop_middleware_ids(),
            active_turn: runtime.inflight_turn_binding().map(|binding| {
                (
                    binding.display_name,
                    binding.thinking,
                    binding.omit_thinking,
                )
            }),
            has_active_turn: runtime.has_active_turn(),
            fixed_context: runtime.fixed_request_context(),
            system_prompt: runtime.system_prompt().map(str::to_string),
            model_list: runtime.model_registry().list().to_vec(),
            capability_pending: capability.pending,
            capability_snapshot: capability.snapshot,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CapabilityView {
    pub pending: bool,
    pub snapshot: xylitol_plugin_sdk::session::CapabilitySnapshot,
}

#[derive(Clone)]
pub struct SharedSubsystems {
    pub store: Option<Arc<dyn xylitol_plugin_sdk::XySessionStore>>,
    pub commands: Arc<Vec<CommandInfo>>,
    pub runtime: Arc<std::sync::Mutex<SharedRuntimeProbe>>,
}

#[derive(Default)]
pub struct SharedRuntimeProbe {
    pub queues: Option<Arc<crate::agent::capabilities::AsyncQueueRuntime>>,
    pub coordinator: Option<state::SharedTurnCoordinator>,
    pub reload_cwd: Option<std::path::PathBuf>,
    pub session_profile: xylitol_plugin_sdk::session::SessionProfile,
    pub profile_mismatch_notice: Option<String>,
    pub prepare_hub: Option<Arc<dyn xylitol_plugin_sdk::SessionPrepareHub>>,
    pub prepare_acked: bool,
    pub bootstrap_settled: bool,
    pub session_id: Option<String>,
    pub tools_frozen: bool,
    pub pending_workspaces: Option<Vec<String>>,
    pub pending_tools: Option<crate::agent::tools::ToolSet>,
    pub builtin_tools: Option<Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>>,
    pub pending_model_registry: Option<crate::agent::model::registry::ModelRegistry>,
    pub todo_gateway: Option<Arc<dyn xylitol_plugin_sdk::AgentTodoGateway>>,
    pub automation_gateway: Option<Arc<dyn xylitol_plugin_sdk::AutomationGateway>>,
    pub workflow_gateway: Option<Arc<dyn xylitol_plugin_sdk::WorkflowGateway>>,
}

impl SharedSubsystems {
    pub fn from_runtime(runtime: &AgentRuntime, commands: Arc<Vec<CommandInfo>>) -> Self {
        let probe = SharedRuntimeProbe {
            queues: Some(runtime.queues()),
            coordinator: Some(runtime.coordinator()),
            reload_cwd: None,
            session_profile: xylitol_plugin_sdk::session::SessionProfile::default(),
            profile_mismatch_notice: None,
            prepare_hub: None,
            prepare_acked: true,
            bootstrap_settled: false,
            session_id: None,
            tools_frozen: false,
            pending_workspaces: None,
            pending_tools: None,
            builtin_tools: None,
            pending_model_registry: None,
            todo_gateway: None,
            automation_gateway: None,
            workflow_gateway: None,
        };
        Self {
            store: Some(runtime.session_store()),
            commands,
            runtime: Arc::new(std::sync::Mutex::new(probe)),
        }
    }

    pub fn store(&self) -> Result<Arc<dyn xylitol_plugin_sdk::XySessionStore>, XyDriverError> {
        self.store
            .clone()
            .ok_or_else(|| XyDriverError::message("no session store bound"))
    }

    pub fn empty(commands: Arc<Vec<CommandInfo>>) -> Self {
        Self {
            store: None,
            commands,
            runtime: Arc::new(std::sync::Mutex::new(SharedRuntimeProbe::default())),
        }
    }

    pub fn set_pending_model_registry(
        &self,
        registry: crate::agent::model::registry::ModelRegistry,
    ) {
        self.probe().pending_model_registry = Some(registry);
    }

    pub fn take_pending_model_registry(
        &self,
    ) -> Option<crate::agent::model::registry::ModelRegistry> {
        self.probe().pending_model_registry.take()
    }

    pub fn set_builtin_tools(&self, tools: Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>) {
        self.probe().builtin_tools = Some(tools);
    }

    pub fn builtin_tools(&self) -> Option<Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>> {
        self.probe().builtin_tools.clone()
    }

    pub fn bind_runtime(&self, runtime: &AgentRuntime) {
        let mut probe = self.probe();
        probe.queues = Some(runtime.queues());
        probe.coordinator = Some(runtime.coordinator());
        probe.session_id = runtime.session_id().map(str::to_string);
        probe.tools_frozen = runtime.is_tools_frozen();
    }

    fn probe(&self) -> std::sync::MutexGuard<'_, SharedRuntimeProbe> {
        self.runtime.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_reload_cwd(&self, cwd: std::path::PathBuf) {
        self.probe().reload_cwd = Some(cwd);
    }

    pub fn reload_cwd(&self) -> Option<std::path::PathBuf> {
        self.probe().reload_cwd.clone()
    }

    pub fn set_session_profile(&self, profile: xylitol_plugin_sdk::session::SessionProfile) {
        self.probe().session_profile = profile;
    }

    pub fn session_profile(&self) -> xylitol_plugin_sdk::session::SessionProfile {
        self.probe().session_profile.clone()
    }

    pub fn set_profile_mismatch_notice(&self, notice: Option<String>) {
        self.probe().profile_mismatch_notice = notice;
    }

    pub fn take_profile_mismatch_notice(&self) -> Option<String> {
        self.probe().profile_mismatch_notice.take()
    }

    pub fn set_prepare_hub(&self, hub: Arc<dyn xylitol_plugin_sdk::SessionPrepareHub>) {
        self.probe().prepare_hub = Some(hub);
    }

    pub fn prepare_hub(&self) -> Option<Arc<dyn xylitol_plugin_sdk::SessionPrepareHub>> {
        self.probe().prepare_hub.clone()
    }

    pub fn set_todo_gateway(&self, gateway: Arc<dyn xylitol_plugin_sdk::AgentTodoGateway>) {
        self.probe().todo_gateway = Some(gateway);
    }

    pub fn set_automation_gateway(&self, gateway: Arc<dyn xylitol_plugin_sdk::AutomationGateway>) {
        self.probe().automation_gateway = Some(gateway);
    }

    pub fn set_workflow_gateway(&self, gateway: Arc<dyn xylitol_plugin_sdk::WorkflowGateway>) {
        self.probe().workflow_gateway = Some(gateway);
    }

    pub async fn bind_session_gateways(&self, session_id: Option<String>) {
        let (todo, automation, workflow) = {
            let probe = self.probe();
            (
                probe.todo_gateway.clone(),
                probe.automation_gateway.clone(),
                probe.workflow_gateway.clone(),
            )
        };
        if let Some(todo) = todo {
            todo.bind_session(session_id.clone()).await;
        }
        if let Some(automation) = automation {
            automation.bind_session(session_id.clone()).await;
        }
        if let Some(workflow) = workflow {
            workflow.bind_session(session_id).await;
        }
    }

    pub fn set_pending_workspaces(&self, workspaces: Vec<String>) {
        self.probe().pending_workspaces = Some(workspaces);
    }

    pub fn take_pending_workspaces(&self) -> Option<Vec<String>> {
        self.probe().pending_workspaces.take()
    }

    pub fn set_pending_tools(&self, tools: crate::agent::tools::ToolSet) {
        self.probe().pending_tools = Some(tools);
    }

    pub fn take_pending_tools(&self) -> Option<crate::agent::tools::ToolSet> {
        self.probe().pending_tools.take()
    }

    pub fn set_tools_frozen(&self, frozen: bool) {
        self.probe().tools_frozen = frozen;
    }

    pub fn tools_frozen(&self) -> bool {
        self.probe().tools_frozen
    }

    pub fn set_session_id(&self, session_id: Option<String>) {
        self.probe().session_id = session_id;
    }

    pub fn session_id(&self) -> Option<String> {
        self.probe().session_id.clone()
    }

    pub fn set_prepare_acked(&self, acked: bool) {
        self.probe().prepare_acked = acked;
    }

    pub fn prepare_acked(&self) -> bool {
        self.probe().prepare_acked
    }

    pub fn mark_bootstrap_settled(&self) {
        self.probe().bootstrap_settled = true;
    }

    pub fn bootstrap_settled(&self) -> bool {
        self.probe().bootstrap_settled
    }

    pub fn fire_prepare(&self, reason: xylitol_plugin_sdk::SessionPrepareReason) {
        let hub = self.probe().prepare_hub.clone();
        if let Some(hub) = hub
            && reason != xylitol_plugin_sdk::SessionPrepareReason::Generate
        {
            hub.cue_connecting();
            hub.fire();
        }
    }

    pub fn release_prepare_session(&self, session_id: &str) {
        if let Some(hub) = self.probe().prepare_hub.clone() {
            hub.release_session(session_id);
        }
    }

    fn queues(&self) -> Option<Arc<AsyncQueueRuntime>> {
        self.probe().queues.clone()
    }

    pub fn steer(&self, message: &str) -> String {
        let Some(queues) = self.queues() else {
            return String::new();
        };
        let coordinator = self.probe().coordinator.clone();
        let turn_in_flight = coordinator
            .map(|coordinator| coordinator.with(|c| c.turn_in_flight()))
            .unwrap_or(false);
        let target = if turn_in_flight {
            &queues.steer
        } else {
            &queues.follow_up
        };
        let item = target
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enqueue(xylitol_plugin_sdk::message::AgentMessage::user(message));
        queues.notify_queue_update();
        item.id
    }

    pub fn follow_up(&self, message: &str) -> String {
        let Some(queues) = self.queues() else {
            return String::new();
        };
        let item = queues
            .follow_up
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enqueue(xylitol_plugin_sdk::message::AgentMessage::user(message));
        queues.notify_queue_update();
        item.id
    }

    pub fn queue_items(&self) -> Vec<xylitol_plugin_sdk::session::QueueItem> {
        self.queues()
            .map(|queues| queues.items())
            .unwrap_or_default()
    }

    pub fn revoke_queue_item(&self, id: &str) -> RevokeOutcome {
        let Some(queues) = self.queues() else {
            return RevokeOutcome {
                revoked: false,
                items: Vec::new(),
            };
        };
        let outcome = queues.revoke(id);
        queues.notify_queue_update();
        outcome
    }

    pub fn queue_stats(&self) -> QueueStats {
        self.queues()
            .map(|queues| queues.stats())
            .unwrap_or_default()
    }

    pub fn clear_steer_queue(&self) {
        let Some(queues) = self.queues() else {
            return;
        };
        queues
            .steer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        queues.notify_queue_update();
    }

    pub fn clear_queues(&self, clear_steer: bool, clear_follow_up: bool) {
        let Some(queues) = self.queues() else {
            return;
        };
        if clear_steer {
            queues
                .steer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
        }
        if clear_follow_up {
            queues
                .follow_up
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
        }
        if clear_steer || clear_follow_up {
            queues.notify_queue_update();
        }
    }

    pub fn abort(&self) {
        if let Some(coordinator) = self.probe().coordinator.clone() {
            coordinator.with_mut(|c| {
                c.cancel_active();
                if let Some(id) = c.phase().active_turn_id() {
                    c.clear_active_turn_if(id);
                }
            });
        }
        self.clear_steer_queue();
    }

    pub fn cancel_token(&self) -> tokio_util::sync::CancellationToken {
        self.probe()
            .coordinator
            .as_ref()
            .map(|coordinator| coordinator.with(|c| c.cancel_token()))
            .unwrap_or_default()
    }

    pub fn phase(&self) -> RuntimePhase {
        self.probe()
            .coordinator
            .as_ref()
            .map(|coordinator| coordinator.with(|c| c.phase()))
            .unwrap_or(RuntimePhase::Idle)
    }

    pub async fn ensure_session(&self, id: &str) -> Result<(), XyDriverError> {
        let store = self.store()?;
        if store.exists(id).await {
            return Ok(());
        }
        store
            .create(id, None, None)
            .await
            .map_err(XyDriverError::from)
    }
}

pub struct ActorState {
    pub runtime: AgentRuntime,
    pub shared: SharedSubsystems,
    pub capability: CapabilityView,
    pub snapshot: DriverSnapshot,
}

pub struct RuntimeSlot(pub std::sync::Mutex<Option<AgentRuntime>>);

impl RuntimeSlot {
    pub fn new(runtime: AgentRuntime) -> Arc<Self> {
        Arc::new(Self(std::sync::Mutex::new(Some(runtime))))
    }

    pub fn take(&self) -> Option<AgentRuntime> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    pub fn put(&self, runtime: AgentRuntime) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(runtime);
    }
}

impl ActorState {
    pub fn new(runtime: AgentRuntime, commands: Arc<Vec<CommandInfo>>) -> Self {
        let shared = SharedSubsystems::from_runtime(&runtime, commands);
        let capability = CapabilityView::default();
        let snapshot = DriverSnapshot::from_runtime(&runtime, capability.clone());
        Self {
            runtime,
            shared,
            capability,
            snapshot,
        }
    }

    pub fn from_slot(slot: &Arc<RuntimeSlot>, shared: SharedSubsystems) -> Option<Self> {
        let runtime = slot.take()?;
        shared.bind_runtime(&runtime);
        let capability = CapabilityView::default();
        let snapshot = DriverSnapshot::from_runtime(&runtime, capability.clone());
        Some(Self {
            runtime,
            shared,
            capability,
            snapshot,
        })
    }

    pub fn into_runtime(self) -> AgentRuntime {
        self.runtime
    }

    pub fn runtime(&self) -> &AgentRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut AgentRuntime {
        &mut self.runtime
    }

    pub fn refresh_snapshot(&mut self) {
        self.snapshot = DriverSnapshot::from_runtime(&self.runtime, self.capability.clone());
    }

    pub fn snapshot(&self) -> DriverSnapshot {
        self.snapshot.clone()
    }
}

pub enum DriverCommand {
    Run {
        parts: Vec<xylitol_plugin_sdk::message::AgentPart>,
        context: Vec<xylitol_plugin_sdk::message::AgentMessage>,
        system_prompt: Option<String>,
        reply: tokio::sync::oneshot::Sender<EventStream>,
    },
    RunQueued {
        reply: tokio::sync::oneshot::Sender<EventStream>,
    },
    SelectModel {
        model_id: String,
        reply: tokio::sync::oneshot::Sender<Result<ModelInfo, XyDriverError>>,
    },
    CycleModel {
        reply: tokio::sync::oneshot::Sender<Result<ModelInfo, XyDriverError>>,
    },
    SetThinkingLevel {
        level: String,
        reply: tokio::sync::oneshot::Sender<Result<(), XyDriverError>>,
    },
    BindSession {
        session_id: String,
        reply: tokio::sync::oneshot::Sender<Result<(), XyDriverError>>,
    },
    SwitchSession {
        session_id: String,
        reply: tokio::sync::oneshot::Sender<Result<String, XyDriverError>>,
    },
    NewSession {
        reply: tokio::sync::oneshot::Sender<Result<String, XyDriverError>>,
    },
    ForkSession {
        entry_id: String,
        position: xylitol_plugin_sdk::session::ForkPosition,
        reply: tokio::sync::oneshot::Sender<Result<String, XyDriverError>>,
    },
    SetSessionName {
        name: String,
        reply: tokio::sync::oneshot::Sender<Result<String, XyDriverError>>,
    },
    SetSessionNameFor {
        session_id: String,
        name: String,
        reply: tokio::sync::oneshot::Sender<Result<String, XyDriverError>>,
    },
    Compact {
        instructions: Option<String>,
        reply: tokio::sync::oneshot::Sender<Result<bool, XyDriverError>>,
    },
    SetTools {
        tools: crate::agent::tools::ToolSet,
        reply: tokio::sync::oneshot::Sender<crate::agent::capabilities::ToolsWriteOutcome>,
    },
    SetWorkspaces {
        workspaces: Vec<String>,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    FreezeTools {
        tools: crate::agent::tools::ToolSet,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    ReopenToolsForRegate {
        reply: tokio::sync::oneshot::Sender<()>,
    },
    BeginToolGating {
        reply: tokio::sync::oneshot::Sender<()>,
    },
    ClearToolFreeze {
        reply: tokio::sync::oneshot::Sender<()>,
    },
    ApplyPromptResources {
        context_files: Vec<(String, String)>,
        system_prompt: Option<String>,
        append_system_prompt: Vec<String>,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    ApplySkills {
        skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    ReplaceModelRegistry {
        registry: crate::agent::model::registry::ModelRegistry,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    ScriptHookCancel {
        event_type: String,
        phase: String,
        context: serde_json::Value,
        reply: tokio::sync::oneshot::Sender<Result<(), XyDriverError>>,
    },
    ScriptHookObserve {
        event_type: String,
        phase: String,
        context: serde_json::Value,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    EnsureSession {
        session_id: String,
        reply: tokio::sync::oneshot::Sender<Result<(), XyDriverError>>,
    },
    BuiltinsForReload {
        reply: tokio::sync::oneshot::Sender<Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>>,
    },
}

pub struct DriverHandle {
    tx: mpsc::Sender<DriverCommand>,
    snapshot: Arc<std::sync::Mutex<DriverSnapshot>>,
    shared: SharedSubsystems,
}

impl std::fmt::Debug for DriverHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriverHandle")
            .field("mailbox_capacity", &MAILBOX_CAPACITY)
            .finish_non_exhaustive()
    }
}

impl Clone for DriverHandle {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            snapshot: Arc::clone(&self.snapshot),
            shared: self.shared.clone(),
        }
    }
}

impl DriverHandle {
    pub fn detached(state: &ActorState) -> Self {
        let (tx, _rx) = mpsc::channel(MAILBOX_CAPACITY);
        Self {
            tx,
            snapshot: Arc::new(std::sync::Mutex::new(state.snapshot())),
            shared: state.shared.clone(),
        }
    }

    pub fn spawn_parked(
        slot: &Arc<RuntimeSlot>,
        shared: SharedSubsystems,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        if let Some(runtime) = slot.take() {
            shared.bind_runtime(&runtime);
            slot.put(runtime);
        }
        let snapshot = Arc::new(std::sync::Mutex::new(DriverSnapshot::default()));
        let (tx, rx) = mpsc::channel(MAILBOX_CAPACITY);
        let slot_clone = Arc::clone(slot);
        let snapshot_clone = Arc::clone(&snapshot);
        let shared_clone = shared.clone();
        let task = tokio::spawn(async move {
            let Some(state) = ActorState::from_slot(&slot_clone, shared_clone) else {
                return;
            };
            run_actor(state, rx, snapshot_clone, slot_clone).await;
        });
        (
            Self {
                tx,
                snapshot,
                shared,
            },
            task,
        )
    }

    pub fn is_spawned(&self) -> bool {
        self.tx.capacity() < MAILBOX_CAPACITY
    }

    pub fn snapshot(&self) -> DriverSnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn shared(&self) -> &SharedSubsystems {
        &self.shared
    }

    pub async fn send(&self, command: DriverCommand) -> Result<(), XyDriverError> {
        self.tx
            .send(command)
            .await
            .map_err(|_| XyDriverError::io("driver actor stopped".to_string()))
    }

    pub async fn request<R>(
        &self,
        build: impl FnOnce(tokio::sync::oneshot::Sender<R>) -> DriverCommand,
    ) -> Result<R, XyDriverError> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.send(build(reply)).await?;
        rx.await
            .map_err(|_| XyDriverError::io("driver actor dropped reply".to_string()))
    }

    pub async fn run_queued(&self) -> EventStream {
        let (reply, rx) = tokio::sync::oneshot::channel();
        if let Err(error) = self.send(DriverCommand::RunQueued { reply }).await {
            return error_stream(error);
        }
        rx.await
            .unwrap_or_else(|_| error_stream(XyDriverError::io("driver actor dropped reply")))
    }

    pub async fn run(
        &self,
        parts: Vec<xylitol_plugin_sdk::message::AgentPart>,
        context: Vec<xylitol_plugin_sdk::message::AgentMessage>,
        system_prompt: Option<String>,
    ) -> EventStream {
        let (reply, rx) = tokio::sync::oneshot::channel();
        if let Err(error) = self
            .send(DriverCommand::Run {
                parts,
                context,
                system_prompt,
                reply,
            })
            .await
        {
            return error_stream(error);
        }
        match rx.await {
            Ok(stream) => stream,
            Err(_) => error_stream(XyDriverError::message("driver actor stopped before run")),
        }
    }
}

async fn run_actor(
    state: ActorState,
    mut rx: mpsc::Receiver<DriverCommand>,
    snapshot: Arc<std::sync::Mutex<DriverSnapshot>>,
    slot: Arc<RuntimeSlot>,
) {
    let shared = state.shared.clone();
    let mut active = Some(state);
    while let Some(command) = rx.recv().await {
        let Some(mut state) = active
            .take()
            .or_else(|| ActorState::from_slot(&slot, shared.clone()))
        else {
            break;
        };
        if let Some(workspaces) = state.shared.take_pending_workspaces() {
            state.runtime.set_workspaces(workspaces);
        }
        if let Some(tools) = state.shared.take_pending_tools() {
            let _ = state.runtime.set_tools(tools);
        }
        if let Some(registry) = state.shared.take_pending_model_registry() {
            state.runtime.replace_model_registry(registry);
        }
        dispatch(&mut state, command).await;
        state
            .shared
            .set_builtin_tools(state.runtime.clone_tools().iter().cloned().collect());
        state
            .shared
            .set_session_id(state.runtime.session_id().map(str::to_string));
        state
            .shared
            .set_tools_frozen(state.runtime.is_tools_frozen());
        state.refresh_snapshot();
        let mut guard = snapshot.lock().unwrap_or_else(|e| e.into_inner());
        *guard = state.snapshot();
        slot.put(state.into_runtime());
    }
}

fn reply<T>(tx: tokio::sync::oneshot::Sender<T>, value: T) {
    tx.send(value).ok();
}

async fn dispatch(state: &mut ActorState, command: DriverCommand) {
    match command {
        DriverCommand::Run {
            parts,
            context,
            system_prompt,
            reply: tx,
        } => {
            let stream = state
                .runtime
                .submit_turn(parts, context, system_prompt)
                .await;
            reply(tx, Box::pin(stream));
        }
        DriverCommand::RunQueued { reply: tx } => {
            let stream = state.runtime.submit_queued().await;
            reply(tx, Box::pin(stream));
        }
        DriverCommand::SelectModel {
            model_id,
            reply: tx,
        } => {
            reply(tx, select_model(state, &model_id).await);
        }
        DriverCommand::CycleModel { reply: tx } => {
            reply(tx, cycle_model(state).await);
        }
        DriverCommand::SetThinkingLevel { level, reply: tx } => {
            reply(
                tx,
                state
                    .runtime
                    .set_thinking_level(level)
                    .await
                    .map_err(XyDriverError::from),
            );
        }
        DriverCommand::BindSession {
            session_id,
            reply: tx,
        } => {
            reply(
                tx,
                state
                    .runtime
                    .bind_session(session_id)
                    .map_err(XyDriverError::from),
            );
        }
        DriverCommand::SwitchSession {
            session_id,
            reply: tx,
        } => {
            reply(tx, actor_switch_session(state, &session_id).await);
        }
        DriverCommand::NewSession { reply: tx } => {
            reply(tx, actor_new_session(state).await);
        }
        DriverCommand::ForkSession {
            entry_id,
            position,
            reply: tx,
        } => {
            reply(tx, actor_fork_session(state, &entry_id, position).await);
        }
        DriverCommand::SetSessionName { name, reply: tx } => {
            reply(tx, actor_set_session_name(state, &name).await);
        }
        DriverCommand::SetSessionNameFor {
            session_id,
            name,
            reply: tx,
        } => {
            reply(
                tx,
                actor_set_session_name_for(state, &session_id, &name).await,
            );
        }
        DriverCommand::Compact {
            instructions,
            reply: tx,
        } => {
            reply(
                tx,
                state
                    .runtime
                    .force_compact(instructions)
                    .await
                    .map(|()| true)
                    .map_err(XyDriverError::from),
            );
        }
        DriverCommand::SetTools { tools, reply: tx } => {
            reply(tx, state.runtime.set_tools(tools));
        }
        DriverCommand::SetWorkspaces {
            workspaces,
            reply: tx,
        } => {
            state.runtime.set_workspaces(workspaces);
            reply(tx, ());
        }
        DriverCommand::FreezeTools { tools, reply: tx } => {
            state.runtime.freeze_tools(tools);
            reply(tx, ());
        }
        DriverCommand::ReopenToolsForRegate { reply: tx } => {
            state.runtime.reopen_tools_for_regate();
            reply(tx, ());
        }
        DriverCommand::BeginToolGating { reply: tx } => {
            state.runtime.begin_tool_gating();
            reply(tx, ());
        }
        DriverCommand::ClearToolFreeze { reply: tx } => {
            state.runtime.clear_tool_freeze();
            reply(tx, ());
        }
        DriverCommand::ApplyPromptResources {
            context_files,
            system_prompt,
            append_system_prompt,
            reply: tx,
        } => {
            state.runtime.apply_prompt_resources(
                context_files,
                system_prompt,
                append_system_prompt,
            );
            reply(tx, ());
        }
        DriverCommand::ApplySkills { skills, reply: tx } => {
            state.runtime.apply_skills(skills);
            reply(tx, ());
        }
        DriverCommand::ReplaceModelRegistry {
            registry,
            reply: tx,
        } => {
            state.runtime.replace_model_registry(registry);
            reply(tx, ());
        }
        DriverCommand::ScriptHookCancel {
            event_type,
            phase,
            context,
            reply: tx,
        } => {
            reply(
                tx,
                state
                    .runtime
                    .script_hook_cancel(&event_type, &phase, context)
                    .await
                    .map_err(XyDriverError::from),
            );
        }
        DriverCommand::ScriptHookObserve {
            event_type,
            phase,
            context,
            reply: tx,
        } => {
            state
                .runtime
                .script_hook_observe(&event_type, &phase, context)
                .await;
            reply(tx, ());
        }
        DriverCommand::EnsureSession {
            session_id,
            reply: tx,
        } => {
            reply(
                tx,
                state
                    .runtime
                    .ensure_session(&session_id, None)
                    .await
                    .map_err(XyDriverError::from),
            );
        }
        DriverCommand::BuiltinsForReload { reply: tx } => {
            reply(tx, state.runtime.clone_tools().iter().cloned().collect());
        }
    }
}

async fn select_model(state: &mut ActorState, model_id: &str) -> Result<ModelInfo, XyDriverError> {
    state
        .runtime
        .select_model(model_id)
        .await
        .map_err(XyDriverError::from)?;
    Ok(state
        .runtime
        .current_model()
        .map(|meta| ModelInfo::from(&meta))
        .unwrap_or_else(|| ModelInfo {
            id: model_id.to_string(),
            display_name: model_id.to_string(),
            thinking: true,
            thinking_levels: Vec::new(),
            context_window: 0,
        }))
}

async fn cycle_model(state: &mut ActorState) -> Result<ModelInfo, XyDriverError> {
    let list = state.runtime.model_registry().list().to_vec();
    if list.is_empty() {
        return Err(XyDriverError::not_found("no models available"));
    }
    let current_id = state.runtime.current_model().map(|meta| meta.id.clone());
    let current_idx = current_id
        .as_ref()
        .and_then(|cur| list.iter().position(|meta| meta.id == *cur))
        .unwrap_or(0);
    let next_idx = (current_idx + 1) % list.len();
    let next_id = list[next_idx].id.clone();
    state
        .runtime
        .select_model_with_source(&next_id, "cycle")
        .await
        .map_err(XyDriverError::from)?;
    Ok(ModelInfo::from(&list[next_idx]))
}

async fn actor_switch_session(
    state: &mut ActorState,
    session_id: &str,
) -> Result<String, XyDriverError> {
    let store = state.shared.store()?;
    if !store.exists(session_id).await {
        return Err(XyDriverError::not_found(session_id.to_string()));
    }
    let fallback_cwd = state.runtime.cwd().to_string();
    store.load_validated(session_id, &fallback_cwd).await?;
    let (ty, phase, ctx) =
        crate::agent::runtime::script_hook_ctx::session_before_switch("resume", session_id);
    state
        .runtime
        .script_hook_cancel(ty, phase, ctx)
        .await
        .map_err(XyDriverError::from)?;
    let previous = state.runtime.session_id().map(str::to_string);
    let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_shutdown_resume(
        session_id,
        previous.as_deref(),
    );
    state.runtime.script_hook_observe(ty, phase, ctx).await;
    crate::driver::in_process::session::migrate_legacy_before_switch(store.as_ref(), session_id)
        .await;
    let context = store.build_session_context(session_id).await?;
    crate::driver::in_process::bind_session_or_err(&mut state.runtime, session_id.to_string())?;
    if let Some(previous) = previous {
        state.shared.release_prepare_session(&previous);
    }
    let stored_profile = store.session_profile(session_id).await?;
    let current_profile = state.shared.session_profile();
    if stored_profile.fingerprint.is_some()
        && stored_profile.fingerprint != current_profile.fingerprint
    {
        state.shared.set_profile_mismatch_notice(Some(format!(
            "agent profile changed since session start: {} -> {}; resume continues without compaction",
            stored_profile.name.as_deref().unwrap_or("unknown"),
            current_profile.name.as_deref().unwrap_or("current")
        )));
    }
    if stored_profile != current_profile {
        store
            .set_session_profile(session_id, current_profile)
            .await?;
    }
    state.runtime.restore_thinking_level(context.thinking_level);
    state.runtime.clear_tool_freeze();
    state.runtime.emit_leaf_changed_settlement().await;
    state
        .shared
        .bind_session_gateways(Some(session_id.to_string()))
        .await;
    state.shared.set_prepare_acked(false);
    state
        .shared
        .fire_prepare(xylitol_plugin_sdk::SessionPrepareReason::Resume);
    Ok(session_id.to_string())
}

async fn actor_new_session(state: &mut ActorState) -> Result<String, XyDriverError> {
    let store = state.shared.store()?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let cwd = state
        .shared
        .reload_cwd()
        .map(|path| path.to_string_lossy().into_owned());
    let cwd = cwd.or_else(|| {
        let agent_cwd = state.runtime.cwd().to_string();
        (!agent_cwd.trim().is_empty()).then_some(agent_cwd)
    });
    store
        .create(&session_id, cwd.as_deref(), None)
        .await
        .map_err(XyDriverError::from)?;
    let profile = state.shared.session_profile();
    if profile.name.is_some() || profile.fingerprint.is_some() {
        store
            .set_session_profile(&session_id, profile)
            .await
            .map_err(XyDriverError::from)?;
    }
    let previous = state.runtime.session_id().map(str::to_string);
    crate::driver::in_process::bind_session_or_err(&mut state.runtime, session_id.clone())?;
    if let Some(previous) = previous {
        state.shared.release_prepare_session(&previous);
    }
    state.runtime.clear_tool_freeze();
    state
        .shared
        .bind_session_gateways(Some(session_id.clone()))
        .await;
    state.shared.set_prepare_acked(false);
    state
        .shared
        .fire_prepare(xylitol_plugin_sdk::SessionPrepareReason::Switch);
    Ok(session_id)
}

async fn actor_fork_session(
    state: &mut ActorState,
    entry_id: &str,
    position: xylitol_plugin_sdk::session::ForkPosition,
) -> Result<String, XyDriverError> {
    let previous = state.runtime.session_id().map(str::to_string);
    let id = state
        .runtime
        .fork_session(entry_id, position)
        .await
        .map_err(XyDriverError::from)?;
    if let Some(previous) = previous {
        state.shared.release_prepare_session(&previous);
    }
    state.runtime.clear_tool_freeze();
    state.shared.bind_session_gateways(Some(id.clone())).await;
    state.shared.set_prepare_acked(false);
    state
        .shared
        .fire_prepare(xylitol_plugin_sdk::SessionPrepareReason::Switch);
    Ok(id)
}

async fn actor_set_session_name(
    state: &mut ActorState,
    name: &str,
) -> Result<String, XyDriverError> {
    let store = state.shared.store()?;
    let session_id = state
        .runtime
        .session_id()
        .map(str::to_string)
        .ok_or_else(|| XyDriverError::message("no active session"))?;
    let out = store
        .set_session_name(&session_id, name)
        .await
        .map_err(XyDriverError::from)?;
    state.runtime.set_session_name_fact(Some(out.clone()));
    xylitol_plugin_sdk::set_obs_session_name(Some(out.as_str()));
    Ok(out)
}

async fn actor_set_session_name_for(
    state: &mut ActorState,
    session_id: &str,
    name: &str,
) -> Result<String, XyDriverError> {
    let store = state.shared.store()?;
    let out = store
        .set_session_name(session_id, name)
        .await
        .map_err(XyDriverError::from)?;
    if state.runtime.session_id() == Some(session_id) {
        state.runtime.set_session_name_fact(Some(out.clone()));
        xylitol_plugin_sdk::set_obs_session_name(Some(out.as_str()));
    }
    Ok(out)
}

fn error_stream(error: XyDriverError) -> EventStream {
    Box::pin(XyEventStream::error(
        xylitol_plugin_sdk::lifecycle::XyEventError::new(
            "DriverActorUnavailable",
            error.to_string(),
        ),
    ))
}
