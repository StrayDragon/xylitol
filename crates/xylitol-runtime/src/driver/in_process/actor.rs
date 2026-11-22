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
}

impl SharedSubsystems {
    pub fn from_runtime(runtime: &AgentRuntime, commands: Arc<Vec<CommandInfo>>) -> Self {
        let probe = SharedRuntimeProbe {
            queues: Some(runtime.queues()),
            coordinator: Some(runtime.coordinator()),
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

    fn probe(&self) -> std::sync::MutexGuard<'_, SharedRuntimeProbe> {
        self.runtime.lock().unwrap_or_else(|e| e.into_inner())
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

    pub fn from_slot(slot: &Arc<RuntimeSlot>, commands: Arc<Vec<CommandInfo>>) -> Option<Self> {
        let runtime = slot.take()?;
        let shared = SharedSubsystems::from_runtime(&runtime, commands);
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
    Compact {
        instructions: Option<String>,
        reply: tokio::sync::oneshot::Sender<Result<bool, XyDriverError>>,
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
        commands: Arc<Vec<CommandInfo>>,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        let shared = slot
            .take()
            .map(|runtime| {
                let shared = SharedSubsystems::from_runtime(&runtime, commands.clone());
                slot.put(runtime);
                shared
            })
            .unwrap_or_else(|| SharedSubsystems::empty(commands.clone()));
        let snapshot = Arc::new(std::sync::Mutex::new(DriverSnapshot::default()));
        let (tx, rx) = mpsc::channel(MAILBOX_CAPACITY);
        let slot_clone = Arc::clone(slot);
        let snapshot_clone = Arc::clone(&snapshot);
        let task = tokio::spawn(async move {
            let Some(state) = ActorState::from_slot(&slot_clone, commands) else {
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
    let mut state = Some(state);
    while let Some(command) = rx.recv().await {
        let Some(mut active) = state.take() else {
            break;
        };
        dispatch(&mut active, command).await;
        active.refresh_snapshot();
        let mut guard = snapshot.lock().unwrap_or_else(|e| e.into_inner());
        *guard = active.snapshot();
        slot.put(active.into_runtime());
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

fn error_stream(error: XyDriverError) -> EventStream {
    Box::pin(XyEventStream::error(
        xylitol_plugin_sdk::lifecycle::XyEventError::new(
            "DriverActorUnavailable",
            error.to_string(),
        ),
    ))
}
