mod assistant;
mod step_end;
pub(crate) mod support;

use crate::utils::{StreamNode, StreamNodeClock};
use assistant::{
    build_assistant_message, current_provider_model, streaming_assistant_parts,
    streaming_message_update, upsert_streaming_tool,
};
use step_end::{
    FinishTurnOutcome, IterationClose, close_iteration, drain_queue, pop_queue, queue_counts,
    requeue_pending, try_turn_end_compaction,
};
use support::{
    ClearActiveTurn, attempt_model_stream, current_model_modality_support, observe_script_hook,
    outbound_has_image, persist_agent_message, persist_agent_message_with_thought_elapsed,
    prepare_turn_binding,
};

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::Stream;
use futures::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::middleware::{
    LOOP_MIDDLEWARE_CONTINUE_LIMIT, MiddlewareFault, fragments_into_prompt, run_after_llm_response,
    run_after_step, run_before_llm_request, run_before_step, run_decide_continue,
};
use super::retry::{RetryState, is_retryable_xy_error};
use super::state::{FrozenRootConfig, RunId, RunLease, RuntimeControlError, SharedRunCoordinator};
use super::{AgentHooks, XyEvent, XyEventStream};
use crate::agent::capabilities::{AgentCapabilities, PendingMessageQueue};
use crate::agent::compaction::CompactionError;
use crate::agent::goal::{GoalDecision, decide_active_goal};
use crate::agent::prompt::{AgentStatusBar, expand_skills_in_agent_messages, project_outbound};
use crate::agent::tools::{ToolCatalog, ToolSet};
use xylitol_plugin_sdk::error::{XyError, XySessionError};
use xylitol_plugin_sdk::message::{AgentMessage, AgentPart};
use xylitol_plugin_sdk::model::{XyChunk, XyModality, XyToolSchema};
use xylitol_plugin_sdk::resource::SkillInfo;
use xylitol_plugin_sdk::{
    AfterLlmResponseCtx, AfterStepCtx, AgentEndReason, BeforeLlmRequestCtx, BeforeStepCtx,
    DecideContinueCtx, LoopChain, LoopControl, XyBatchMode, XyHookBus, XyHookOutcome, XyModel,
    XySessionStore, XyStream,
};

pub struct AgentRuntime {
    inner: AgentCapabilities,
    coordinator: SharedRunCoordinator,
    pending_prepare: std::sync::Mutex<Option<xylitol_plugin_sdk::SessionPrepareSnapshot>>,
}

impl AgentRuntime {
    pub fn new(mut inner: AgentCapabilities) -> Self {
        let coordinator = SharedRunCoordinator::new();
        let midturn_coord = coordinator.clone();
        let turn_flight_coord = coordinator.clone();
        inner.set_midturn_active_probe(Arc::new(move || {
            midturn_coord.with(|c| c.has_active_run() || c.has_active_turn())
        }));
        inner.set_turn_in_flight_probe(Arc::new(move || {
            turn_flight_coord.with(|c| c.turn_in_flight())
        }));
        Self {
            inner,
            coordinator,
            pending_prepare: std::sync::Mutex::new(None),
        }
    }

    pub fn set_pending_prepare(&self, snap: xylitol_plugin_sdk::SessionPrepareSnapshot) {
        if let Ok(mut g) = self.pending_prepare.lock() {
            *g = Some(snap);
        }
    }

    pub fn bind_session(
        &mut self,
        session_id: impl Into<String>,
    ) -> Result<(), RuntimeControlError> {
        if self.coordinator.with(|c| c.has_work()) {
            return Err(RuntimeControlError::SessionBusy);
        }
        self.inner.set_session(session_id.into());
        Ok(())
    }

    pub fn set_session_name_fact(&mut self, name: Option<String>) {
        self.inner.set_session_name_fact(name);
    }

    pub fn set_stage_name_config(&mut self, config: crate::agent::stage_name::StageNameConfig) {
        self.inner.set_stage_name_config(config);
    }

    pub async fn generate_session_name(
        &mut self,
        force: bool,
    ) -> Result<String, xylitol_plugin_sdk::error::XyError> {
        self.inner.generate_session_name(force).await
    }

    pub fn session_id(&self) -> Option<&str> {
        self.inner.session_id()
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.coordinator.with(|c| c.cancel_token())
    }

    pub fn abort(&self) {
        self.coordinator.with_mut(|c| {
            c.cancel_active();
            if let Some(id) = c.phase().active_run_id() {
                c.clear_active_turn_if(id);
            }
        });
        self.inner.clear_steer_queue();
    }

    pub fn steer(&self, message: impl Into<String>) {
        self.inner.steer(message);
    }

    pub fn follow_up(&self, message: impl Into<String>) {
        self.inner.follow_up(message);
    }

    pub fn clear_queues(&self, clear_steer: bool, clear_follow_up: bool) {
        self.inner.clear_queues(clear_steer, clear_follow_up);
    }

    pub fn queue_stats(&self) -> crate::agent::capabilities::QueueStats {
        self.inner.queue_stats()
    }

    pub fn session_store(&self) -> Arc<dyn xylitol_plugin_sdk::XySessionStore> {
        self.inner.session_store()
    }

    pub fn set_tools(&mut self, tools: ToolSet) -> crate::agent::capabilities::ToolsWriteOutcome {
        self.inner.set_tools(tools)
    }

    pub fn set_tools_defer_prompt(
        &mut self,
        tools: ToolSet,
    ) -> crate::agent::capabilities::ToolsWriteOutcome {
        self.inner.set_tools_defer_prompt(tools)
    }

    pub fn install_system_prompt_text(&mut self, prompt: String) {
        self.inner.install_system_prompt_text(prompt);
    }

    pub fn tool_freeze_phase(&self) -> crate::agent::tools::ToolFreezePhase {
        self.inner.tool_freeze_phase()
    }

    pub fn is_tools_frozen(&self) -> bool {
        self.inner.is_tools_frozen()
    }

    pub fn frozen_tool_fingerprint(&self) -> Option<crate::agent::tools::ToolTableFingerprint> {
        self.inner.frozen_tool_fingerprint().cloned()
    }

    pub fn begin_tool_gating(&mut self) {
        self.inner.begin_tool_gating();
    }

    pub fn reopen_tools_for_regate(&mut self) {
        self.inner.reopen_tools_for_regate();
    }

    pub fn clear_tool_freeze(&mut self) {
        self.inner.clear_tool_freeze();
    }

    pub fn freeze_tools(&mut self, tools: ToolSet) {
        self.inner.freeze_tools(tools);
    }

    pub fn clone_tools(&self) -> ToolSet {
        self.inner.tools().clone()
    }

    pub fn freeze_tools_defer_prompt(
        &mut self,
        tools: ToolSet,
    ) -> crate::agent::prompt::SystemPromptOpts {
        self.inner.freeze_tools_defer_prompt(tools)
    }

    pub fn frozen_fingerprint_matches_set(&self, candidate: &ToolSet) -> bool {
        self.inner.frozen_fingerprint_matches_set(candidate)
    }

    pub fn set_loop_middleware(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
    ) {
        self.inner.set_loop_registry(registry);
        self.inner
            .set_loop_scope(xylitol_plugin_sdk::LoopScope::Root);
    }

    pub fn set_loop_middleware_scoped(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
        scope: xylitol_plugin_sdk::LoopScope,
    ) {
        self.inner.set_loop_registry(registry);
        self.inner.set_loop_scope(scope);
    }

    pub fn loop_middleware_ids(&self) -> Vec<String> {
        self.inner
            .loop_registry()
            .map(|registry| registry.ids())
            .unwrap_or_default()
    }

    pub fn replace_hooks(&mut self, hooks: AgentHooks) {
        self.inner.replace_hooks(hooks);
    }

    pub fn add_hook(&mut self, hook: super::hooks::BeforeToolHook) {
        self.inner.hooks_mut().add_before(hook);
    }

    pub fn set_should_stop_after_step(
        &mut self,
        hook: Option<super::hooks::ShouldStopAfterStepHook>,
    ) {
        self.inner.hooks_mut().set_should_stop_after_step(hook);
    }

    pub fn set_permission(&mut self, permission: Arc<dyn xylitol_plugin_sdk::XyPermission>) {
        self.inner.set_permission(permission);
    }

    pub fn set_tool_mode(&mut self, mode: xylitol_plugin_sdk::XyBatchMode) {
        self.inner.set_tool_mode(mode);
    }

    pub fn set_batch_mode(&mut self, mode: xylitol_plugin_sdk::XyBatchMode) {
        self.inner.set_tool_mode(mode);
    }

    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.inner.set_system_prompt(prompt);
    }

    pub fn apply_prompt_resources(
        &mut self,
        context_files: Vec<(String, String)>,
        system_prompt: Option<String>,
        append_system_prompt: Vec<String>,
    ) {
        self.inner
            .apply_prompt_resources(context_files, system_prompt, append_system_prompt);
    }

    pub fn apply_skills(&mut self, skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>) {
        self.inner.apply_skills(skills);
    }

    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.inner.loaded_skill_names()
    }

    pub fn loaded_skills(&self) -> Vec<xylitol_plugin_sdk::resource::SkillInfo> {
        self.inner.loaded_skills().to_vec()
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.inner.system_prompt()
    }

    pub fn fixed_request_context(&self) -> crate::agent::compaction::FixedRequestContext {
        self.inner.fixed_request_context()
    }

    pub async fn emit_leaf_changed_settlement(&self) {
        self.inner.emit_leaf_changed_settlement().await
    }

    pub fn tools_snapshot(&self) -> ToolSet {
        self.inner.tools().clone()
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.inner
            .tools()
            .iter()
            .map(|t| t.name().to_string())
            .collect()
    }

    pub fn cwd(&self) -> &str {
        self.inner.cwd()
    }

    pub fn set_cwd(&mut self, cwd: impl Into<String>) {
        self.inner.set_cwd(cwd);
    }

    pub fn workspaces(&self) -> &[String] {
        self.inner.workspaces()
    }

    pub fn set_workspaces(&mut self, workspaces: Vec<String>) {
        self.inner.set_workspaces(workspaces);
    }

    pub fn hook_bus(&self) -> Option<Arc<dyn XyHookBus>> {
        self.inner.hook_bus()
    }

    pub async fn script_hook_cancel(
        &self,
        event_type: &str,
        phase: &str,
        context: serde_json::Value,
    ) -> Result<(), crate::agent::capabilities::HookBlockedError> {
        match self.inner.hook_bus() {
            Some(bus) => {
                crate::agent::capabilities::cancel_hook(&bus, event_type, phase, context).await
            }
            None => Ok(()),
        }
    }

    pub async fn script_hook_observe(
        &self,
        event_type: &str,
        phase: &str,
        context: serde_json::Value,
    ) {
        if let Some(bus) = self.inner.hook_bus() {
            crate::agent::capabilities::observe_hook(&bus, event_type, phase, context).await;
        }
    }

    pub fn current_model(&self) -> Option<xylitol_plugin_sdk::model::XyModelMeta> {
        self.inner.current_model()
    }

    pub fn model_registry(&self) -> crate::agent::model::registry::ModelRegistry {
        self.inner.model_registry()
    }

    pub fn replace_model_registry(
        &mut self,
        registry: crate::agent::model::registry::ModelRegistry,
    ) {
        self.inner.replace_model_registry(registry);
    }

    pub async fn select_model(&mut self, model_id: &str) -> Result<(), XyError> {
        self.inner.select_model(model_id).await
    }

    pub async fn select_model_with_source(
        &mut self,
        model_id: &str,
        source: &str,
    ) -> Result<(), XyError> {
        self.inner.select_model_with_source(model_id, source).await
    }

    pub fn thinking_level(&self) -> String {
        self.inner.thinking_level()
    }

    pub async fn set_thinking_level(&mut self, level: String) -> Result<(), XyError> {
        self.inner.set_thinking_level(level).await
    }

    pub fn apply_default_thinking_level(&mut self, raw: Option<&str>) {
        self.inner.apply_default_thinking_level(raw);
    }

    pub fn set_thinking_budgets(
        &mut self,
        budgets: Option<xylitol_plugin_sdk::model::ThinkingBudgets>,
    ) {
        self.inner.set_thinking_budgets(budgets);
    }

    pub fn restore_thinking_level(&mut self, level: String) {
        self.inner.restore_thinking_level(level);
    }

    pub fn inflight_turn_binding(&self) -> Option<crate::agent::capabilities::ActiveTurnBinding> {
        self.coordinator.with(|c| c.inflight_turn_binding())
    }

    pub fn has_active_turn(&self) -> bool {
        self.coordinator
            .with(|c| c.has_active_turn() || c.has_active_run())
    }

    pub fn active_turn_binding(&self) -> Option<crate::agent::capabilities::ActiveTurnBinding> {
        if let Some(active) = self.inflight_turn_binding() {
            return Some(active);
        }
        self.inner.selected_turn_binding()
    }

    pub async fn ensure_session(&self, id: &str, parent: Option<&str>) -> Result<(), XyError> {
        self.inner.ensure_session(id, parent).await
    }

    pub async fn fork_session(
        &self,
        at_entry_id: &str,
        position: xylitol_plugin_sdk::session::ForkPosition,
    ) -> Result<String, XyError> {
        if self.coordinator.with(|c| c.has_work()) {
            return Err(XyError::from(XySessionError::busy(
                "session mutation unavailable while busy",
            )));
        }
        self.inner.fork_session(at_entry_id, position).await
    }

    pub async fn get_session_stats(
        &self,
    ) -> Result<crate::agent::capabilities::SessionStats, XyError> {
        self.inner.get_session_stats().await
    }

    pub async fn force_compact(&self, instructions: Option<String>) -> Result<(), CompactionError> {
        if self.coordinator.with(|c| c.has_work()) {
            return Err(CompactionError::policy("compact unavailable while busy"));
        }
        self.inner.force_compact(instructions).await
    }

    pub async fn maybe_auto_compact(&self) -> Result<bool, CompactionError> {
        self.inner.maybe_auto_compact().await
    }

    pub async fn submit_root(&mut self, prompt: &str) -> XyEventStream {
        self.submit_parts(vec![xylitol_plugin_sdk::message::AgentPart::text(prompt)])
            .await
    }

    pub async fn submit_parts(
        &mut self,
        parts: Vec<xylitol_plugin_sdk::message::AgentPart>,
    ) -> XyEventStream {
        let Some(session_id) = self.inner.session_id().map(str::to_string) else {
            return XyEventStream::error(xylitol_plugin_sdk::lifecycle::XyEventError::new(
                "NoSession",
                "no session bound; call bind_session first",
            ));
        };

        let frozen = self.freeze_root_config(parts);

        enum Admit {
            Immediate {
                run_id: RunId,
                cancel: CancellationToken,
            },
            Busy,
        }

        let admit = self.coordinator.with_mut(|c| {
            if c.phase().is_idle() {
                let (run_id, cancel) = c.try_begin_immediate().expect("idle begin");
                return Admit::Immediate { run_id, cancel };
            }
            Admit::Busy
        });

        match admit {
            Admit::Busy => XyEventStream::busy(),
            Admit::Immediate { run_id, cancel } => {
                self.start_root_stream(session_id, run_id, cancel, frozen)
                    .await
            }
        }
    }

    fn freeze_root_config(&self, user_parts: Vec<AgentPart>) -> FrozenRootConfig {
        FrozenRootConfig {
            user_parts,
            system_prompt: self.inner.system_prompt().map(str::to_string),
            tools: self.inner.tools().clone(),
            hooks: self.inner.hooks().clone(),
            batch_mode: self.inner.tool_mode(),
            skills: self.inner.loaded_skills().to_vec(),
            compaction_settings: self.inner.compaction_settings(),
            compaction: self.inner.compaction(),
            permission: self.inner.get_permission(),
            hook_bus: self.inner.hook_bus(),
            cwd: self.inner.cwd().to_string(),
            workspaces: self.inner.workspaces().to_vec(),
            pending_prepare: self.pending_prepare.lock().ok().and_then(|g| g.clone()),
            loop_chain: self.inner.loop_chain(),
        }
    }

    async fn start_root_stream(
        &self,
        session_id: String,
        run_id: RunId,
        cancel: CancellationToken,
        frozen: FrozenRootConfig,
    ) -> XyEventStream {
        let t_ensure = std::time::Instant::now();
        if let Err(e) = self.inner.ensure_session(&session_id, None).await {
            self.coordinator.with_mut(|c| {
                let _ = c.finish_run(run_id);
            });
            return XyEventStream::error(xylitol_plugin_sdk::lifecycle::XyEventError::from_xy(&e));
        }
        {
            let ms = t_ensure.elapsed().as_millis();
            if ms >= 16 {
                log::info!(target: "xylitol::lag", "run_ensure_session {ms}ms");
            } else {
                log::debug!(target: "xylitol::lag", "run_ensure_session {ms}ms");
            }
        }

        let t_hist = std::time::Instant::now();
        let seeded_history = match self.inner.load_conversation_history(&session_id).await {
            Ok(h) => h,
            Err(e) => {
                self.coordinator.with_mut(|c| {
                    let _ = c.finish_run(run_id);
                });
                return XyEventStream::error(xylitol_plugin_sdk::lifecycle::XyEventError::from_xy(
                    &e,
                ));
            }
        };
        {
            let ms = t_hist.elapsed().as_millis();
            let n = seeded_history.len();
            if ms >= 16 {
                log::info!(target: "xylitol::lag", "run_load_history {ms}ms entries={n}");
            } else {
                log::debug!(target: "xylitol::lag", "run_load_history {ms}ms entries={n}");
            }
        }

        let queues = self.inner.queues();
        let lease = RunLease::new(self.coordinator.clone(), run_id, Some(queues.clone()));
        let stream = build_live_react_stream(LiveReactArgs {
            run_id,
            coordinator: self.coordinator.clone(),
            cancel,
            frozen,
            model_manager: self.inner.model_manager_handle(),
            queues,
            store: self.inner.session_store(),
            session_id,
            seeded_history,
            event_sink_inner: self.inner.event_sink(),
            obs_session_name: self.inner.session_name().map(str::to_string),
        });
        self.coordinator.with_mut(|c| c.mark_streaming(run_id));
        XyEventStream::with_lease(Box::pin(stream), lease)
    }
}

struct LiveReactArgs {
    run_id: RunId,
    coordinator: SharedRunCoordinator,
    cancel: CancellationToken,
    frozen: FrozenRootConfig,
    model_manager: Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    queues: Arc<crate::agent::capabilities::AsyncQueueRuntime>,
    store: Arc<dyn XySessionStore>,
    session_id: String,
    seeded_history: Vec<AgentMessage>,
    event_sink_inner: Arc<dyn xylitol_plugin_sdk::XyEventSink>,
    obs_session_name: Option<String>,
}

fn build_live_react_stream(args: LiveReactArgs) -> impl Stream<Item = XyEvent> + Send {
    let LiveReactArgs {
        run_id,
        coordinator,
        cancel,
        frozen,
        model_manager,
        queues,
        store,
        session_id,
        seeded_history,
        event_sink_inner,
        obs_session_name,
    } = args;

    let FrozenRootConfig {
        user_parts,
        system_prompt,
        tools,
        hooks,
        batch_mode,
        skills,
        compaction_settings,
        compaction,
        permission,
        hook_bus,
        cwd,
        workspaces,
        pending_prepare,
        loop_chain,
    } = frozen;

    type PermissionCheck = Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>;
    let permission_check: Option<PermissionCheck> = Some(Arc::new(
        move |tool_name: &str, tool_path: &str| -> Option<String> {
            super::permission_router::check_tool_permission(
                permission.as_ref(),
                tool_name,
                tool_path,
            )
        },
    ));

    let t_schemas = std::time::Instant::now();
    let tool_schemas: Vec<XyToolSchema> = tools
        .iter()
        .filter(|tool| tool.exposure().model_visible())
        .map(|t| XyToolSchema {
            name: t.name().to_string(),
            description: t.description().to_string(),
            parameters: t.parameters_schema(),
        })
        .collect();
    let tool_catalog: Arc<[xylitol_plugin_sdk::XyToolDescriptor]> =
        Arc::from(ToolCatalog::from_toolset(&tools, None).descriptors());
    {
        let ms = t_schemas.elapsed().as_millis();
        let n = tool_schemas.len();
        if ms >= 16 {
            log::info!(target: "xylitol::lag", "run_build_tool_schemas {ms}ms tools={n}");
        } else {
            log::debug!(target: "xylitol::lag", "run_build_tool_schemas {ms}ms tools={n}");
        }
    }

    let steer_queue = queues.steer.clone();
    let follow_up_queue = queues.follow_up.clone();
    let (side_tx, side_rx) = tokio::sync::mpsc::unbounded_channel::<XyEvent>();
    let event_sink: Arc<dyn xylitol_plugin_sdk::XyEventSink> = Arc::new(CompactionStreamTee {
        inner: event_sink_inner,
        tx: side_tx,
    });

    let (queue_tx, queue_rx) = tokio::sync::mpsc::unbounded_channel();
    queues.bind_event_tx(run_id, queue_tx);

    let react = Box::pin(run_react_loop(ReActConfig {
        run_id,
        coordinator,
        model_manager,
        system_prompt,
        tools,
        tool_schemas,
        tool_catalog,
        user_parts,
        cancel,
        permission_check,
        hooks,
        hook_bus,
        batch_mode,
        steer_queue,
        follow_up_queue,
        store,
        session_id,
        seeded_history,
        skills,
        event_sink,
        compaction_settings,
        compaction,
        cwd,
        workspaces,
        obs_session_name,
        pending_prepare,
        loop_chain,
    }));

    combine_run_streams(react, side_rx, queue_rx)
}

pub(crate) fn combine_run_streams(
    react: Pin<Box<dyn Stream<Item = XyEvent> + Send>>,
    mut side_rx: tokio::sync::mpsc::UnboundedReceiver<XyEvent>,
    mut queue_rx: tokio::sync::mpsc::UnboundedReceiver<XyEvent>,
) -> impl Stream<Item = XyEvent> + Send {
    async_stream::stream! {
        let mut react = react;
        loop {
            tokio::select! {
                biased;
                ev = react.next() => {
                    match ev {
                        Some(e) => {
                            if matches!(e, XyEvent::AgentEnd { .. }) {
                                while let Ok(ev2) = side_rx.try_recv() {
                                    yield ev2;
                                }
                                while let Ok(ev2) = queue_rx.try_recv() {
                                    yield ev2;
                                }
                            }
                            yield e;
                        }
                        None => {
                            while let Ok(ev2) = side_rx.try_recv() {
                                yield ev2;
                            }
                            while let Ok(ev2) = queue_rx.try_recv() {
                                yield ev2;
                            }
                            break;
                        }
                    }
                }
                ev = queue_rx.recv() => {
                    if let Some(e) = ev {
                        yield e;
                    }
                }
                ev = side_rx.recv() => {
                    if let Some(e) = ev {
                        yield e;
                    }
                }
            }
        }
    }
}

struct CompactionStreamTee {
    inner: Arc<dyn xylitol_plugin_sdk::XyEventSink>,
    tx: tokio::sync::mpsc::UnboundedSender<XyEvent>,
}

#[async_trait::async_trait]
impl xylitol_plugin_sdk::XyEventSink for CompactionStreamTee {
    async fn emit(&self, event: &XyEvent) {
        self.inner.emit(event).await;
        if matches!(
            event,
            XyEvent::CompactionStart { .. }
                | XyEvent::CompactionEnd { .. }
                | XyEvent::ContextTokenSettlement { .. }
        ) {
            let _ = self.tx.send(event.clone());
        }
    }
}

struct ReActConfig {
    run_id: RunId,
    coordinator: SharedRunCoordinator,
    model_manager: Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    system_prompt: Option<String>,
    tools: ToolSet,
    tool_schemas: Vec<XyToolSchema>,
    tool_catalog: Arc<[xylitol_plugin_sdk::XyToolDescriptor]>,
    user_parts: Vec<xylitol_plugin_sdk::message::AgentPart>,
    cancel: CancellationToken,
    #[allow(clippy::type_complexity)]
    permission_check: Option<std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>>,
    hooks: AgentHooks,
    hook_bus: Option<Arc<dyn XyHookBus>>,
    batch_mode: XyBatchMode,
    steer_queue: Arc<Mutex<PendingMessageQueue>>,
    follow_up_queue: Arc<Mutex<PendingMessageQueue>>,
    store: Arc<dyn XySessionStore>,
    session_id: String,
    seeded_history: Vec<AgentMessage>,
    skills: Vec<SkillInfo>,
    event_sink: Arc<dyn xylitol_plugin_sdk::XyEventSink>,
    compaction_settings: crate::agent::compaction::CompactionSettings,
    compaction: std::sync::Arc<dyn crate::agent::compaction::XyCompaction>,
    cwd: String,
    workspaces: Vec<String>,
    obs_session_name: Option<String>,
    pending_prepare: Option<xylitol_plugin_sdk::SessionPrepareSnapshot>,
    loop_chain: LoopChain,
}

#[allow(clippy::too_many_arguments)]
async fn after_step_or_fault(
    chain: &LoopChain,
    cancel: &CancellationToken,
    parent: Option<fastrace::prelude::SpanContext>,
    obs: &xylitol_plugin_sdk::ObsSessionContext,
    session_id: &str,
    turn: usize,
    continued_tools: bool,
    history_len: usize,
) -> Option<MiddlewareFault> {
    if chain.after_step.is_empty() {
        return None;
    }
    run_after_step(
        chain,
        cancel,
        parent,
        obs,
        AfterStepCtx {
            step_index: turn as u32,
            continued_tools,
            history_len,
            session_id: Some(session_id.to_string()),
        },
    )
    .await
    .err()
}

fn middleware_limit_event(decision: &str) -> XyEvent {
    XyEvent::Error(xylitol_plugin_sdk::lifecycle::XyEventError::new(
        "LoopMiddlewareLimit",
        format!(
            "loop middleware exceeded {LOOP_MIDDLEWARE_CONTINUE_LIMIT} \
             non-progressing {decision} decisions"
        ),
    ))
}

fn note_abort(reason: AgentEndReason, slot: &mut Option<AgentEndReason>) {
    if slot.is_none() {
        *slot = Some(reason);
    }
}

fn middleware_fault_event(fault: &MiddlewareFault) -> XyEvent {
    if fault.kind() == "Aborted" {
        XyEvent::aborted()
    } else {
        XyEvent::Error(xylitol_plugin_sdk::lifecycle::XyEventError::new(
            fault.kind(),
            fault.message(),
        ))
    }
}

async fn run_turn_injection(
    chain: &LoopChain,
    ctx: &BeforeStepCtx,
    history: &mut Vec<AgentMessage>,
) {
    for injector in &chain.turn_injectors {
        for message in injector.on_turn_submission(ctx).await {
            history.push(message);
        }
    }
}

fn run_react_loop(cfg: ReActConfig) -> impl Stream<Item = XyEvent> + Send {
    let ReActConfig {
        run_id,
        coordinator,
        model_manager,
        system_prompt,
        tools,
        tool_schemas,
        tool_catalog,
        user_parts,
        mut cancel,
        permission_check,
        hooks,
        hook_bus,
        batch_mode,
        steer_queue,
        follow_up_queue,
        store,
        session_id,
        seeded_history,
        skills,
        event_sink,
        compaction_settings,
        compaction,
        cwd,
        workspaces,
        obs_session_name,
        pending_prepare,
        loop_chain,
    } = cfg;
    let mut fixed_context = crate::agent::compaction::FixedRequestContext {
        system_prompt: system_prompt.clone(),
        tool_schemas: tool_schemas.clone(),
    };
    async_stream::stream! {
        let _clear_active = ClearActiveTurn {
            coordinator: coordinator.clone(),
            run_id,
        };

        if let Some(bus) = &hook_bus {
            let (ty, phase, ctx) = super::script_hook_ctx::turn_start();
            observe_script_hook(bus, ty, phase, ctx).await;
        }

        let mut history: Vec<AgentMessage> = seeded_history;
        let run_baseline = history.len();

        let env_snap = crate::agent::prompt::snapshot_for_cwd(&cwd);
        if crate::agent::prompt::ensure_session_env_in_history(&mut history, &env_snap) {
            persist_agent_message(&store, &session_id, history.last().expect("session_env")).await;
        }
        if let Some(prep) = pending_prepare.as_ref()
            && xylitol_plugin_sdk::ensure_session_prepare_in_history(&mut history, prep)
        {
            persist_agent_message(
                &store,
                &session_id,
                history.last().expect("session_prepare"),
            )
            .await;
        }

        let frozen_tools = crate::agent::tools::ToolTableFingerprint::from_toolset(&tools);
        let tools_fingerprint = xylitol_plugin_sdk::ToolFingerprint {
            names: frozen_tools.names,
            content_digest: frozen_tools.content_digest,
        };

        if !loop_chain.turn_injectors.is_empty() {
            let submission_ctx = BeforeStepCtx {
                step_index: 0,
                has_new_user_messages: true,
                system_prompt: system_prompt.clone(),
                system_fragments: Vec::new(),
                skills: skills.clone(),
                tools_fingerprint: Some(tools_fingerprint.clone()),
                prepare_fingerprint: pending_prepare
                    .as_ref()
                    .map(|prepare| prepare.fingerprint.clone()),
                session_id: Some(session_id.clone()),
            };
            run_turn_injection(&loop_chain, &submission_ctx, &mut history).await;
        }

        history.push(AgentMessage::user_parts(user_parts.clone()));
        persist_agent_message(&store, &session_id, history.last().expect("user message")).await;

        let mut pending: Vec<AgentMessage> = drain_queue(&steer_queue);
        let mut turn: usize = 0;
        let mut run_model: Option<(String, Arc<dyn XyModel>)> = None;
        let mut overflow_recovery_attempted = false;
        let mut compaction_floor_notice_emitted = false;
        let mut stream_clock = StreamNodeClock::new();
        stream_clock.stamp(StreamNode::AgentStart);
        let user_preview = super::tool_exec::parts_preview_text(&user_parts);
        let (model_api, run_model_id) = {
            let mm = crate::utils::lock_mutex(&model_manager);
            match mm.current_model() {
                Some(m) => (Some(m.api.clone()), m.id.clone()),
                None => (None, String::new()),
            }
        };
        let (parent_session_id, fork_at_entry_id) = store
            .load_entries(&session_id)
            .await
            .ok()
            .map(|entries| xylitol_plugin_sdk::session::session_fork_edge(&entries))
            .unwrap_or((None, None));
        let obs_session = xylitol_plugin_sdk::ObsSessionContext {
            session_id: Some(session_id.clone()),
            session_name: obs_session_name,
            parent_session_id,
            fork_at_entry_id,
            llm_gateway_session_id: None,
            ..Default::default()
        };
        let agent_turn_span = super::obs::AgentTurnSpan::start_with_session(
            Some(user_preview.as_str()),
            model_api.as_deref(),
            &obs_session,
        );
        let turn_obs_parent = super::tool_exec::capture_iteration_parent(
            agent_turn_span.as_ref().map(|s| s.span()),
        );
        let mut abort_reason: Option<AgentEndReason> = None;
        let mut step_consumes_new_user = true;
        let chain_active = !loop_chain.is_empty();
        let decide_active = !loop_chain.decide_continue.is_empty();
        let mut stall_count: u32 = 0;
        let mut settle_count: u32 = 0;
        coordinator.with_mut(|c| c.set_turn_in_flight(true));
        yield XyEvent::AgentStart {
            session_id: session_id.clone(),
            model: run_model_id,
        };

        'outer: loop {
            let mut continue_after_tools = true;

            'steps: while continue_after_tools || !pending.is_empty() {
                if cancel.is_cancelled() {
                    note_abort(AgentEndReason::UserCancel, &mut abort_reason);
                    yield XyEvent::aborted();
                    break 'steps;
                }

                let mut turn_system_prompt = system_prompt.clone();
                if chain_active {
                    let ctx = BeforeStepCtx {
                        step_index: turn as u32,
                        has_new_user_messages: step_consumes_new_user || !pending.is_empty(),
                        system_prompt: system_prompt.clone(),
                        system_fragments: Vec::new(),
                        skills: skills.clone(),
                        tools_fingerprint: Some(tools_fingerprint.clone()),
                        prepare_fingerprint: pending_prepare
                            .as_ref()
                            .map(|prepare| prepare.fingerprint.clone()),
                        session_id: Some(session_id.clone()),
                    };
                    match run_before_step(&loop_chain, &cancel, turn_obs_parent, &obs_session, ctx).await {
                        Ok(out) => {
                            turn_system_prompt = fragments_into_prompt(
                                system_prompt.clone(),
                                &out.ctx.system_fragments,
                            );
                            fixed_context.system_prompt = turn_system_prompt.clone();
                            if out.control != LoopControl::Continue {
                                note_abort(AgentEndReason::MiddlewareStop, &mut abort_reason);
                                break 'outer;
                            }
                        }
                        Err(fault) => {
                            note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                            yield middleware_fault_event(&fault);
                            break 'outer;
                        }
                    }
                }

                step_consumes_new_user = false;
                stream_clock.begin_turn();
                stream_clock.stamp(StreamNode::StepStart);
                yield XyEvent::StepStart { step_index: turn as u32 };
                if let Some(bus) = &hook_bus {
                    let (ty, phase, ctx) = super::script_hook_ctx::step_start(turn as u32);
                    observe_script_hook(bus, ty, phase, ctx).await;
                }
                let iteration_span =
                    super::obs::AgentIterationSpan::start(agent_turn_span.as_ref(), turn);
                let iteration_parent = super::tool_exec::capture_iteration_parent(
                    iteration_span.as_ref().map(|s| s.span()),
                );
                let turn_id = iteration_span
                    .as_ref()
                    .map(|t| t.turn_id().to_string())
                    .or_else(|| agent_turn_span.as_ref().map(|t| t.turn_id().to_string()));

                if !pending.is_empty() {
                    for message in pending.drain(..) {
                        yield XyEvent::MessageStart {
                            role: "user".to_string(),
                            message: Some(message.clone()),
                        };
                        if let Some(bus) = &hook_bus {
                            let (ty, phase, ctx) = super::script_hook_ctx::message_start("user");
                            observe_script_hook(bus, ty, phase, ctx).await;
                        }
                        persist_agent_message(&store, &session_id, &message).await;
                        yield XyEvent::MessageEnd {
                            role: "user".to_string(),
                            message: Some(message.clone()),
                        };
                        if let Some(bus) = &hook_bus {
                            let (ty, phase, ctx) = super::script_hook_ctx::message_end("user");
                            observe_script_hook(bus, ty, phase, ctx).await;
                        }
                        history.push(message.clone());
                    }
                    let (steer_count, follow_up_count) =
                        queue_counts(&steer_queue, &follow_up_queue);
                    yield XyEvent::QueueUpdate {
                        steer_count,
                        follow_up_count,
                    };
                }

                let mut messages = history.clone();
                expand_skills_in_agent_messages(&mut messages, &skills);
                if let Some(bus) = &hook_bus
                    && let (ty, phase, ctx) =
                        super::script_hook_ctx::context_pre(messages.len())
                    && let XyHookOutcome::Blocked { reason } =
                        bus.dispatch(ty, phase, ctx).await
                {
                    note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                    yield XyEvent::error_msg(format!("context hook blocked: {reason}"));
                    break 'outer;
                }

                let (model, mut generate_options) = match prepare_turn_binding(
                    &model_manager,
                    &coordinator,
                    run_id,
                    &turn_system_prompt,
                    &mut run_model,
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        note_abort(AgentEndReason::ModelError, &mut abort_reason);
                        yield XyEvent::error_msg(format!("model build error: {e}"));
                        break 'outer;
                    }
                };
                generate_options.obs_parent = iteration_parent;
                generate_options.obs_session = obs_session.clone();

                let bar = match store.load_leaf_branch(&session_id).await {
                    Ok(branch) => AgentStatusBar::from_session_entries(&branch),
                    Err(_) => AgentStatusBar::default(),
                };
                let mut turn_tool_schemas = tool_schemas.clone();
                if chain_active {
                    let ctx = BeforeLlmRequestCtx {
                        step_index: turn as u32,
                        messages: messages.clone(),
                        tool_schemas: turn_tool_schemas.clone(),
                        tool_catalog: tool_catalog.clone(),
                        options: generate_options.clone(),
                        todo: bar.todo().cloned(),
                        session_id: Some(session_id.clone()),
                    };
                    match run_before_llm_request(&loop_chain, &cancel, iteration_parent, &obs_session, ctx).await {
                        Ok(out) => {
                            messages = out.ctx.messages;
                            turn_tool_schemas = out.ctx.tool_schemas;
                            generate_options = out.ctx.options;
                            generate_options.obs_parent = iteration_parent;
                            generate_options.obs_session = obs_session.clone();
                            fixed_context.system_prompt = generate_options.system_prompt.clone();
                            fixed_context.tool_schemas = turn_tool_schemas.clone();
                            if out.control != LoopControl::Continue {
                                yield XyEvent::StepEnd {
                                    step_index: turn as u32,
                                };
                                if let Some(fault) = after_step_or_fault(
                                    &loop_chain,
                                    &cancel,
                                    turn_obs_parent,
                                    &obs_session,
                                    &session_id,
                                    turn,
                                    false,
                                    history.len(),
                                )
                                .await
                                {
                                    note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                    yield middleware_fault_event(&fault);
                                }
                                note_abort(AgentEndReason::MiddlewareStop, &mut abort_reason);
                                break 'outer;
                            }
                        }
                        Err(fault) => {
                            note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                            yield middleware_fault_event(&fault);
                            break 'outer;
                        }
                    }
                }

                let llm_messages = project_outbound(&messages, &bar);
                if outbound_has_image(&llm_messages)
                    && let Some((model_id, supported)) =
                        current_model_modality_support(&model_manager, XyModality::Image)
                    && !supported
                {
                    note_abort(AgentEndReason::ModalityUnsupported, &mut abort_reason);
                    yield XyEvent::error_msg(format!(
                        "model {model_id} does not declare image input; set models.{model_id}.input_modalities to include image"
                    ));
                    break 'outer;
                }
                let mut stream_result: Option<Result<XyStream, XyError>> = None;
                let retry_state = RetryState::new(3, 1000);
                loop {
                    let result = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => break,
                        result = attempt_model_stream(
                            &model, llm_messages.clone(), &turn_tool_schemas, &generate_options,
                        ) => result,
                    };
                    match result {
                        Ok(stream) => {
                            if retry_state.attempt() > 0 {
                                yield XyEvent::AutoRetryEnd {
                                    success: true,
                                    attempt: retry_state.attempt(),
                                };
                            }
                            stream_result = Some(Ok(stream));
                            break;
                        }
                        Err(e) => {
                            if is_retryable_xy_error(&e) && retry_state.can_retry() {
                                log::warn!(
                                    target: "xylitol::react",
                                    "model.generate_stream retrying error.kind={} turn_id={} error={e}",
                                    e.kind(),
                                    turn_id.as_deref().unwrap_or("")
                                );
                                let delay = retry_state
                                    .next_delay_with_hint(e.provider_detail().and_then(|d| d.retry_after));
                                yield XyEvent::AutoRetryStart {
                                    attempt: retry_state.attempt(),
                                    max_retries: retry_state.max_retries(),
                                    delay_ms: delay.as_millis() as u64,
                                };
                                tokio::select! {
                                    biased;
                                    _ = cancel.cancelled() => break,
                                    _ = retry_state.backoff(delay) => continue,
                                }
                            }
                            super::obs::record_xy_error(
                                "model.generate_stream",
                                &e,
                                turn_id.as_deref(),
                                generate_options.obs_parent,
                                &obs_session,
                            );
                            if retry_state.attempt() > 0 {
                                yield XyEvent::AutoRetryEnd {
                                    success: false,
                                    attempt: retry_state.attempt(),
                                };
                            }
                            stream_result = Some(Err(e));
                            break;
                        }
                    }
                }

                let mut chunk_stream: Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;
                match stream_result {
                    None => {
                        note_abort(AgentEndReason::UserCancel, &mut abort_reason);
                        yield XyEvent::aborted();
                        break 'steps;
                    }
                    Some(Ok(s)) => {
                        chunk_stream = s;
                    }
                    Some(Err(e)) => {
                        let err_text = e.to_string();
                        let (provider, model_id) = current_provider_model(&model_manager);
                        let err_asst = build_assistant_message(
                            vec![AgentPart::text("")],
                            Some(xylitol_plugin_sdk::message::XyStopReason::Error),
                            None,
                            provider,
                            model_id,
                            Some(err_text.clone()),
                        );
                        persist_agent_message(&store, &session_id, &err_asst).await;
                        history.push(err_asst);
                        yield XyEvent::error_msg(err_text);
                        let will_continue = try_turn_end_compaction(
                            &store,
                            &session_id,
                            &model_manager,
                            &event_sink,
                            &compaction_settings,
                            compaction.as_ref(),
                            &fixed_context,
                            &mut history,
                            &mut overflow_recovery_attempted,
                            &mut compaction_floor_notice_emitted,
                            None,
                            turn_obs_parent,
                            &obs_session,
                            &cwd,
                            pending_prepare.as_ref(),
                        )
                        .await;
                        if will_continue {
                            continue 'outer;
                        }
                        note_abort(AgentEndReason::ModelError, &mut abort_reason);
                        break 'outer;
                    }
                }

                yield XyEvent::MessageStart {
                    role: "assistant".to_string(),
                    message: None,
                };
                if let Some(bus) = &hook_bus {
                    let (ty, phase, ctx) = super::script_hook_ctx::message_start("assistant");
                    observe_script_hook(bus, ty, phase, ctx).await;
                }

                let mut text_acc = String::new();
                let mut thinking_acc = String::new();
                let mut thinking_signature: Option<String> = None;
                let mut tool_calls: Vec<(String, String, Value)> = Vec::new();
                let mut done_usage: Option<xylitol_plugin_sdk::message::XyUsage> = None;
                let mut done_stop_reason: Option<xylitol_plugin_sdk::message::XyStopReason> = None;
                let mut done_error_message: Option<String> = None;
                #[cfg(debug_assertions)]
                let mut debug_chunks: Vec<XyChunk> = Vec::new();
                #[cfg(debug_assertions)]
                let mut debug_stream_error = false;
                #[cfg(debug_assertions)]
                let mut debug_stream_complete = false;

                loop {
                    let chunk_result = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => {
                            drop(chunk_stream);
                            let assistant_parts = streaming_assistant_parts(
                                &text_acc,
                                &thinking_acc,
                                thinking_signature.as_deref(),
                                &tool_calls,
                            );
                            if !assistant_parts.is_empty() {
                                let assistant_msg = build_assistant_message(
                                    assistant_parts,
                                    Some(xylitol_plugin_sdk::message::XyStopReason::Aborted),
                                    done_usage.take(),
                                    String::new(),
                                    String::new(),
                                    None,
                                );
                                stream_clock.finish_message();
                                persist_agent_message_with_thought_elapsed(
                                    &store,
                                    &session_id,
                                    &assistant_msg,
                                    Some(&stream_clock),
                                )
                                .await;
                                yield XyEvent::MessageEnd {
                                    role: "assistant".to_string(),
                                    message: Some(assistant_msg.clone()),
                                };
                                history.push(assistant_msg);
                            }
                            note_abort(AgentEndReason::UserCancel, &mut abort_reason);
                            yield XyEvent::aborted();
                            break 'steps;
                        }
                        next = chunk_stream.next() => next,
                    };
                    match chunk_result {
                        None => {
                            #[cfg(debug_assertions)]
                            {
                                debug_stream_complete = true;
                            }
                            break;
                        }
                        Some(Ok(chunk)) => {
                            #[cfg(debug_assertions)]
                            debug_chunks.push(chunk.clone());
                            match chunk {
                            XyChunk::TextDelta(text) => {
                                stream_clock.on_text_delta();
                                text_acc.push_str(&text);
                                yield XyEvent::TextDelta(text.clone());
                                yield streaming_message_update(
                                    &text_acc,
                                    &thinking_acc,
                                    thinking_signature.as_deref(),
                                    &tool_calls,
                                );
                            }
                            XyChunk::ThinkingDelta(text) => {
                                stream_clock.on_thinking_delta();
                                thinking_acc.push_str(&text);
                                yield XyEvent::ThinkingDelta(text);
                                yield streaming_message_update(
                                    &text_acc,
                                    &thinking_acc,
                                    thinking_signature.as_deref(),
                                    &tool_calls,
                                );
                            }
                            XyChunk::ThinkingEnd {
                                thinking,
                                thinking_signature: sig,
                            } => {
                                stream_clock.on_thinking_end_chunk();
                                if !thinking.is_empty() {
                                    thinking_acc = thinking;
                                }
                                if sig.is_some() {
                                    thinking_signature = sig;
                                }
                                yield streaming_message_update(
                                    &text_acc,
                                    &thinking_acc,
                                    thinking_signature.as_deref(),
                                    &tool_calls,
                                );
                            }
                            XyChunk::ToolCallStart { id, name } => {
                                stream_clock.on_tool_intent();
                                upsert_streaming_tool(
                                    &mut tool_calls,
                                    id,
                                    name,
                                    Value::Object(Default::default()),
                                );
                                yield streaming_message_update(
                                    &text_acc,
                                    &thinking_acc,
                                    thinking_signature.as_deref(),
                                    &tool_calls,
                                );
                            }
                            XyChunk::ToolCallDelta {
                                id,
                                name,
                                args,
                                ..
                            } => {
                                stream_clock.on_tool_intent();
                                upsert_streaming_tool(&mut tool_calls, id, name, args);
                                yield streaming_message_update(
                                    &text_acc,
                                    &thinking_acc,
                                    thinking_signature.as_deref(),
                                    &tool_calls,
                                );
                            }
                            XyChunk::ToolCallEnd { name, args, id } => {
                                stream_clock.on_tool_intent();
                                upsert_streaming_tool(&mut tool_calls, id, name, args);
                                yield streaming_message_update(
                                    &text_acc,
                                    &thinking_acc,
                                    thinking_signature.as_deref(),
                                    &tool_calls,
                                );
                            }
                            XyChunk::Done {
                                finish_reason,
                                usage,
                            } => {
                                done_stop_reason = Some(finish_reason);
                                if usage.is_some() {
                                    done_usage = usage;
                                }
                            }
                            }
                        }
                        Some(Err(e)) => {
                            #[cfg(debug_assertions)]
                            {
                                debug_stream_error = true;
                            }
                            super::obs::record_xy_error(
                                "model.stream",
                                &e,
                                turn_id.as_deref(),
                                iteration_parent,
                                &obs_session,
                            );
                            let err_text = format!("stream error: {e}");
                            done_stop_reason =
                                Some(xylitol_plugin_sdk::message::XyStopReason::Error);
                            done_error_message = Some(err_text.clone());
                            yield XyEvent::error_msg(err_text);
                            break;
                        }
                    }
                }

                #[cfg(debug_assertions)]
                if debug_stream_complete && !debug_stream_error && !debug_chunks.is_empty() {
                    let outcome = xylitol_plugin_sdk::StreamOutcome::new(debug_chunks);
                    if let Err(err) = outcome.validate() {
                        debug_assert!(
                            false,
                            "provider chunk stream violated terminal contract: {err}"
                        );
                    }
                }

                let mut assistant_parts = streaming_assistant_parts(
                    &text_acc,
                    &thinking_acc,
                    thinking_signature.as_deref(),
                    &tool_calls,
                );
                let mut assistant_msg: Option<AgentMessage> = if !assistant_parts.is_empty()
                    || matches!(
                        done_stop_reason,
                        Some(xylitol_plugin_sdk::message::XyStopReason::Error)
                    )
                {
                    let (provider, model_id) = current_provider_model(&model_manager);
                    if assistant_parts.is_empty() {
                        assistant_parts.push(AgentPart::text(""));
                    }
                    Some(build_assistant_message(
                        assistant_parts,
                        done_stop_reason,
                        done_usage,
                        provider,
                        model_id,
                        done_error_message,
                    ))
                } else {
                    None
                };

                if chain_active
                    && let Some(msg) = assistant_msg.take()
                {
                    let ctx = AfterLlmResponseCtx {
                        step_index: turn as u32,
                        assistant: msg,
                        session_id: Some(session_id.clone()),
                    };
                    match run_after_llm_response(&loop_chain, &cancel, iteration_parent, &obs_session, ctx).await {
                        Ok(out) => {
                            let msg = out.ctx.assistant;
                            tool_calls = msg
                                .content()
                                .iter()
                                .filter_map(|part| match part {
                                    AgentPart::ToolCall {
                                        id,
                                        name,
                                        arguments,
                                    } => Some((id.clone(), name.clone(), arguments.clone())),
                                    _ => None,
                                })
                                .collect();
                            assistant_msg = Some(msg);
                            if out.control != LoopControl::Continue {
                                let msg = assistant_msg.expect("assistant after middleware");
                                stream_clock.finish_message();
                                persist_agent_message_with_thought_elapsed(
                                    &store,
                                    &session_id,
                                    &msg,
                                    Some(&stream_clock),
                                )
                                .await;
                                yield XyEvent::MessageEnd {
                                    role: "assistant".to_string(),
                                    message: Some(msg.clone()),
                                };
                                history.push(msg);
                                yield XyEvent::StepEnd {
                                    step_index: turn as u32,
                                };
                                if let Some(fault) = after_step_or_fault(
                                    &loop_chain,
                                    &cancel,
                                    turn_obs_parent,
                                    &obs_session,
                                    &session_id,
                                    turn,
                                    false,
                                    history.len(),
                                )
                                .await
                                {
                                    note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                    yield middleware_fault_event(&fault);
                                }
                                note_abort(AgentEndReason::MiddlewareStop, &mut abort_reason);
                                break 'outer;
                            }
                        }
                        Err(fault) => {
                            note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                            yield middleware_fault_event(&fault);
                            break 'outer;
                        }
                    }
                }

                if let Some(msg) = assistant_msg.as_ref() {
                    stream_clock.finish_message();
                    persist_agent_message_with_thought_elapsed(
                        &store,
                        &session_id,
                        msg,
                        Some(&stream_clock),
                    )
                    .await;
                }
                yield XyEvent::MessageEnd {
                    role: "assistant".to_string(),
                    message: assistant_msg.clone(),
                };
                if let Some(msg) = assistant_msg {
                    history.push(msg);
                }
                if let Some(bus) = &hook_bus {
                    let (ty, phase, ctx) = super::script_hook_ctx::message_end("assistant");
                    observe_script_hook(bus, ty, phase, ctx).await;
                }

                continue_after_tools = !tool_calls.is_empty();

                if tool_calls.is_empty() {
                    let assistant = history
                        .iter()
                        .rev()
                        .find(|m| m.role_name() == "assistant")
                        .cloned();
                    let finished = close_iteration(
                        &store,
                        &session_id,
                        &model_manager,
                        &event_sink,
                        &compaction_settings,
                        compaction.as_ref(),
                        &fixed_context,
                        &mut history,
                        &mut overflow_recovery_attempted,
                        &mut compaction_floor_notice_emitted,
                        &hooks,
                        &hook_bus,
                        turn,
                        run_baseline,
                        assistant,
                        Vec::new(),
                        &steer_queue,
                        &follow_up_queue,
                        turn_obs_parent,
                        &obs_session,
                        &cwd,
                        IterationClose::Settle,
                        pending_prepare.as_ref(),
                    )
                    .await;
                    for event in finished.events {
                        yield event;
                    }
                    let settle_outcome = finished.outcome;
                    if let Some(fault) =
                        after_step_or_fault(&loop_chain, &cancel, turn_obs_parent, &obs_session, &session_id, turn, false, history.len())
                            .await
                    {
                        note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                        yield middleware_fault_event(&fault);
                        if let FinishTurnOutcome::Advanced {
                            pending: drained,
                            ..
                        } = settle_outcome
                            && let Some((steer_count, follow_up_count)) =
                                requeue_pending(&steer_queue, &follow_up_queue, drained)
                        {
                            yield XyEvent::QueueUpdate {
                                steer_count,
                                follow_up_count,
                            };
                        }
                        break 'outer;
                    }
                    if !matches!(settle_outcome, FinishTurnOutcome::Advanced { .. }) {
                        match settle_outcome {
                            FinishTurnOutcome::ContinueOuterForCompaction => continue 'outer,
                            FinishTurnOutcome::StopRun => {
                                note_abort(AgentEndReason::StepLimit, &mut abort_reason);
                                break 'outer;
                            }
                            FinishTurnOutcome::Advanced { .. } => unreachable!(),
                        }
                    }
                    let FinishTurnOutcome::Advanced {
                        pending: next_pending,
                        queue_update,
                    } = settle_outcome
                    else {
                        unreachable!()
                    };
                    let has_new_user = !next_pending.is_empty();
                    let goal_control = match decide_active_goal(
                        &store,
                        &session_id,
                        &history,
                        &model_manager,
                        &cancel,
                    )
                    .await
                    {
                        Ok(control) => control,
                        Err(error) => {
                            note_abort(AgentEndReason::ModelError, &mut abort_reason);
                            yield XyEvent::error_msg(format!("goal evaluator failed: {error}"));
                            if let Some((steer_count, follow_up_count)) =
                                requeue_pending(&steer_queue, &follow_up_queue, next_pending)
                            {
                                yield XyEvent::QueueUpdate {
                                    steer_count,
                                    follow_up_count,
                                };
                            }
                            break 'outer;
                        }
                    };
                    let mut goal_handled = false;
                    match goal_control {
                        Some(GoalDecision::Continue { guidance }) => {
                            goal_handled = true;
                            let message = AgentMessage::user_parts(vec![AgentPart::text(format!(
                                "<system-reminder>目标尚未达成，继续推进。\n{guidance}</system-reminder>"
                            ))]);
                            crate::utils::lock_mutex(&follow_up_queue).enqueue(message);
                        }
                        Some(GoalDecision::Stop) => {
                            note_abort(AgentEndReason::MiddlewareStop, &mut abort_reason);
                            if let Some((steer_count, follow_up_count)) =
                                requeue_pending(&steer_queue, &follow_up_queue, next_pending)
                            {
                                yield XyEvent::QueueUpdate {
                                    steer_count,
                                    follow_up_count,
                                };
                            }
                            break 'outer;
                        }
                        None => {}
                    }
                    if !goal_handled && decide_active {
                        let ctx = DecideContinueCtx {
                            step_index: turn as u32,
                            settle_count,
                            has_new_user_messages: has_new_user,
                            assistant: history
                                .iter()
                                .rev()
                                .find(|m| m.role_name() == "assistant")
                                .cloned(),
                            history_len: history.len(),
                            compact: None,
                        };
                        match run_decide_continue(&loop_chain, &cancel, turn_obs_parent, &obs_session, ctx).await {
                            Ok(out) => match out.control {
                                LoopControl::Continue => {
                                    if has_new_user {
                                        stall_count = 0;
                                    } else {
                                        stall_count += 1;
                                    }
                                    continue_after_tools = true;
                                }
                                LoopControl::Stop => {
                                    note_abort(AgentEndReason::MiddlewareStop, &mut abort_reason);
                                    if let Some((steer_count, follow_up_count)) =
                                        requeue_pending(&steer_queue, &follow_up_queue, next_pending)
                                    {
                                        yield XyEvent::QueueUpdate {
                                            steer_count,
                                            follow_up_count,
                                        };
                                    }
                                    break 'outer;
                                }
                                LoopControl::Compact => {
                                    if has_new_user {
                                        stall_count = 0;
                                    } else {
                                        stall_count += 1;
                                    }
                                    if stall_count > LOOP_MIDDLEWARE_CONTINUE_LIMIT {
                                        note_abort(AgentEndReason::MiddlewareStop, &mut abort_reason);
                                        yield middleware_limit_event("compact");
                                        if let Some((steer_count, follow_up_count)) =
                                            requeue_pending(&steer_queue, &follow_up_queue, next_pending)
                                        {
                                            yield XyEvent::QueueUpdate {
                                                steer_count,
                                                follow_up_count,
                                            };
                                        }
                                        break 'outer;
                                    }
                                    let request = out.ctx.compact.unwrap_or_default();
                                    let requested_by = out
                                        .stopped_by
                                        .clone()
                                        .unwrap_or_else(|| "unknown".into());
                                    let reason = request
                                        .reason
                                        .unwrap_or_else(|| format!("loop_middleware:{requested_by}"));
                                    match super::react::step_end::force_compact_once(
                                        &store,
                                        &session_id,
                                        &model_manager,
                                        &event_sink,
                                        compaction.as_ref(),
                                        &fixed_context,
                                        &obs_session,
                                        &cwd,
                                        pending_prepare.as_ref(),
                                        request.instructions,
                                        &reason,
                                    )
                                    .await
                                    {
                                        Ok(reloaded) => {
                                            history = reloaded;
                                            pending = next_pending;
                                            turn += 1;
                                            settle_count += 1;
                                            if let Some((steer_count, follow_up_count)) = queue_update {
                                                yield XyEvent::QueueUpdate {
                                                    steer_count,
                                                    follow_up_count,
                                                };
                                            }
                                            continue 'outer;
                                        }
                                        Err(e) => {
                                            note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                            yield XyEvent::error_msg(format!(
                                                "loop middleware {requested_by} compact failed: {e}"
                                            ));
                                            if let Some((steer_count, follow_up_count)) =
                                                requeue_pending(&steer_queue, &follow_up_queue, next_pending)
                                            {
                                                yield XyEvent::QueueUpdate {
                                                    steer_count,
                                                    follow_up_count,
                                                };
                                            }
                                            break 'outer;
                                        }
                                    }
                                }
                                LoopControl::Handoff => {
                                    yield XyEvent::Error(
                                        xylitol_plugin_sdk::lifecycle::XyEventError::new(
                                            "LoopMiddlewareHandoffUnsupported",
                                            format!(
                                                "loop middleware {} requested handoff; not supported",
                                                out.stopped_by.unwrap_or_else(|| "unknown".into())
                                            ),
                                        ),
                                    );
                                    note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                    if let Some((steer_count, follow_up_count)) =
                                        requeue_pending(&steer_queue, &follow_up_queue, next_pending)
                                    {
                                        yield XyEvent::QueueUpdate {
                                            steer_count,
                                            follow_up_count,
                                        };
                                    }
                                    break 'outer;
                                }
                            },
                            Err(fault) => {
                                note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                yield middleware_fault_event(&fault);
                                if let Some((steer_count, follow_up_count)) =
                                    requeue_pending(&steer_queue, &follow_up_queue, next_pending)
                                {
                                    yield XyEvent::QueueUpdate {
                                        steer_count,
                                        follow_up_count,
                                    };
                                }
                                break 'outer;
                            }
                        }
                        if stall_count > LOOP_MIDDLEWARE_CONTINUE_LIMIT {
                            note_abort(AgentEndReason::MiddlewareStop, &mut abort_reason);
                            yield middleware_limit_event("continue");
                            if let Some((steer_count, follow_up_count)) =
                                requeue_pending(&steer_queue, &follow_up_queue, next_pending)
                            {
                                yield XyEvent::QueueUpdate {
                                    steer_count,
                                    follow_up_count,
                                };
                            }
                            break 'outer;
                        }
                    }
                    turn += 1;
                    settle_count += 1;
                    pending = next_pending;
                    if let Some((steer_count, follow_up_count)) = queue_update {
                        yield XyEvent::QueueUpdate {
                            steer_count,
                            follow_up_count,
                        };
                    }
                    continue;
                }

                let mut turn_tool_results: Vec<AgentMessage> = Vec::new();
                let turn_assistant = history.last().cloned();

                let tool_env = super::tool_exec::ToolExecEnv {
                    tools: &tools,
                    hooks: &hooks,
                    hook_bus: &hook_bus,
                    permission_check: &permission_check,
                    cancel: &cancel,
                    turn_id: turn_id.as_deref(),
                    batch_mode,
                    workspace: &cwd,
                    workspaces: &workspaces,
                    obs_session: &obs_session,
                    loop_chain: &loop_chain,
                    step_index: turn as u32,
                };
                let parent_ctx = iteration_parent;

                let mut prepared: Vec<super::tool_exec::PreparedCall> =
                    Vec::with_capacity(tool_calls.len());
                for (id, name, args) in &tool_calls {
                    prepared.push(
                        super::tool_exec::prepare_call(&tool_env, id, name, args, parent_ctx).await,
                    );
                }

                let planned: Vec<super::tool_batch::PlannedWindow> = match batch_mode {
                    XyBatchMode::Sequential => (0..prepared.len())
                        .map(super::tool_batch::PlannedWindow::Barrier)
                        .collect(),
                    XyBatchMode::BarrierParallel => {
                        let classes: Vec<_> = prepared
                            .iter()
                            .map(|call| {
                                super::tool_batch::classify(
                                    &call.name,
                                    call.tool
                                        .as_ref()
                                        .map(|t| t.as_ref() as &dyn xylitol_plugin_sdk::XyTool),
                                )
                            })
                            .collect();
                        super::tool_batch::plan_windows(&classes)
                    }
                };

                for (barrier_index, window) in planned.into_iter().enumerate() {
                    let barrier_index = barrier_index as u32;
                    match window {
                        super::tool_batch::PlannedWindow::Barrier(i) => {
                            let call = &prepared[i];
                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                            let fut = super::tool_exec::run_one(
                                &tool_env,
                                call,
                                tx,
                                parent_ctx,
                                barrier_index,
                            );
                            tokio::pin!(fut);
                            let msg = loop {
                                tokio::select! {
                                    biased;
                                    ev = rx.recv() => {
                                        match ev {
                                            Some(e) => yield e,
                                            None => break fut.await,
                                        }
                                    }
                                    result = &mut fut => {
                                        while let Ok(e) = rx.try_recv() {
                                            yield e;
                                        }
                                        break result;
                                    }
                                }
                            };
                            let outcome = msg;
                            history.push(outcome.message.clone());
                            persist_agent_message(
                                &store,
                                &session_id,
                                history.last().expect("tool result"),
                            )
                            .await;
                            turn_tool_results.push(outcome.message);
                            if outcome.stop_run {
                                yield XyEvent::StepEnd {
                                    step_index: turn as u32,
                                };
                                if let Some(fault) = after_step_or_fault(
                                    &loop_chain,
                                    &cancel,
                                    turn_obs_parent,
                                    &obs_session,
                                    &session_id,
                                    turn,
                                    true,
                                    history.len(),
                                )
                                .await
                                {
                                    note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                    yield middleware_fault_event(&fault);
                                } else if let Some(fault) = &outcome.fault {
                                    note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                    yield middleware_fault_event(fault);
                                }
                                note_abort(AgentEndReason::MiddlewareStop, &mut abort_reason);
                                break 'outer;
                            }
                        }
                        super::tool_batch::PlannedWindow::Parallel(idxs) => {
                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                            let mut futs = Vec::with_capacity(idxs.len());
                            for &i in &idxs {
                                let call = &prepared[i];
                                let tx = tx.clone();
                                futs.push(async {
                                    super::tool_exec::run_one(
                                        &tool_env,
                                        call,
                                        tx,
                                        parent_ctx,
                                        barrier_index,
                                    )
                                    .await
                                });
                            }
                            drop(tx);
                            let join = futures::future::join_all(futs);
                            tokio::pin!(join);
                            let msgs = loop {
                                tokio::select! {
                                    biased;
                                    ev = rx.recv() => {
                                        match ev {
                                            Some(e) => yield e,
                                            None => break join.await,
                                        }
                                    }
                                    results = &mut join => {
                                        while let Ok(e) = rx.try_recv() {
                                            yield e;
                                        }
                                        break results;
                                    }
                                }
                            };
                            for outcome in msgs {
                                history.push(outcome.message.clone());
                                persist_agent_message(
                                    &store,
                                    &session_id,
                                    history.last().expect("tool result"),
                                )
                                .await;
                                turn_tool_results.push(outcome.message);
                                if outcome.stop_run {
                                    yield XyEvent::StepEnd {
                                        step_index: turn as u32,
                                    };
                                    if let Some(fault) = after_step_or_fault(
                                        &loop_chain,
                                        &cancel,
                                        turn_obs_parent,
                                        &obs_session,
                                        &session_id,
                                        turn,
                                        true,
                                        history.len(),
                                    )
                                    .await
                                    {
                                        note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                        yield middleware_fault_event(&fault);
                                    } else if let Some(fault) = &outcome.fault {
                                        note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                                        yield middleware_fault_event(fault);
                                    }
                                    break 'outer;
                                }
                            }
                        }
                    }
                }

                let finished = close_iteration(
                    &store,
                    &session_id,
                    &model_manager,
                    &event_sink,
                    &compaction_settings,
                    compaction.as_ref(),
                    &fixed_context,
                    &mut history,
                    &mut overflow_recovery_attempted,
                    &mut compaction_floor_notice_emitted,
                    &hooks,
                    &hook_bus,
                    turn,
                    run_baseline,
                    turn_assistant,
                    turn_tool_results,
                    &steer_queue,
                    &follow_up_queue,
                    turn_obs_parent,
                    &obs_session,
                    &cwd,
                    IterationClose::ContinueTools,
                    pending_prepare.as_ref(),
                )
                .await;
                for event in finished.events {
                    yield event;
                }
                if let Some(fault) =
                    after_step_or_fault(&loop_chain, &cancel, turn_obs_parent, &obs_session, &session_id, turn, true, history.len())
                        .await
                {
                    note_abort(AgentEndReason::MiddlewareFault, &mut abort_reason);
                    yield middleware_fault_event(&fault);
                    break 'outer;
                }
                stall_count = 0;
                match finished.outcome {
                    FinishTurnOutcome::ContinueOuterForCompaction => continue 'outer,
                    FinishTurnOutcome::StopRun => {
                        note_abort(AgentEndReason::StepLimit, &mut abort_reason);
                        break 'outer;
                    }
                    FinishTurnOutcome::Advanced {
                        pending: next_pending,
                        queue_update,
                    } => {
                        turn += 1;
                        pending = next_pending;
                        if let Some((steer_count, follow_up_count)) = queue_update {
                            yield XyEvent::QueueUpdate {
                                steer_count,
                                follow_up_count,
                            };
                        }
                    }
                }
            }

            // NOTE: 本 turn 到达边界（settle 或 abort 完成），turn 不再在途；steer 自此刻起改走 follow_up FIFO
            coordinator.with_mut(|c| c.set_turn_in_flight(false));
            let turn_reason = abort_reason;
            abort_reason = None;
            let Some(next_submission) = pop_queue(&follow_up_queue) else {
                abort_reason = turn_reason;
                break;
            };
            turn = 0;
            settle_count = 0;
            stall_count = 0;
            if turn_reason.is_some() {
                // NOTE: abort 只针对被取消的 turn；后续 turn 换发全新取消令牌，流继续弹出下一条
                cancel = coordinator
                    .with_mut(|c| c.rearm_cancel_after_abort(run_id))
                    .unwrap_or_else(CancellationToken::new);
            }
            if let Some(bus) = &hook_bus {
                let (ty, phase, ctx) = super::script_hook_ctx::turn_settled();
                observe_script_hook(bus, ty, phase, ctx).await;
                let (ty, phase, ctx) = super::script_hook_ctx::turn_end();
                observe_script_hook(bus, ty, phase, ctx).await;
            }
            yield XyEvent::AgentEnd {
                messages: history.clone(),
                reason: turn_reason,
            };
            if let Some(bus) = &hook_bus {
                let (ty, phase, ctx) = super::script_hook_ctx::turn_start();
                observe_script_hook(bus, ty, phase, ctx).await;
            }
            let turn_model_id = {
                let mm = crate::utils::lock_mutex(&model_manager);
                mm.current_model()
                    .map(|model| model.id.clone())
                    .unwrap_or_default()
            };
            stream_clock.stamp(StreamNode::AgentStart);
            coordinator.with_mut(|c| c.set_turn_in_flight(true));
            yield XyEvent::AgentStart {
                session_id: session_id.clone(),
                model: turn_model_id,
            };
            if !loop_chain.turn_injectors.is_empty() {
                let submission_ctx = BeforeStepCtx {
                    step_index: 0,
                    has_new_user_messages: true,
                    system_prompt: system_prompt.clone(),
                    system_fragments: Vec::new(),
                    skills: skills.clone(),
                    tools_fingerprint: Some(tools_fingerprint.clone()),
                    prepare_fingerprint: pending_prepare
                        .as_ref()
                        .map(|prepare| prepare.fingerprint.clone()),
                    session_id: Some(session_id.clone()),
                };
                run_turn_injection(&loop_chain, &submission_ctx, &mut history).await;
            }
            pending = vec![next_submission];
            let (steer_count, follow_up_count) = queue_counts(&steer_queue, &follow_up_queue);
            yield XyEvent::QueueUpdate {
                steer_count,
                follow_up_count,
            };
        }

        if let Some(bus) = &hook_bus {
            let (ty, phase, ctx) = super::script_hook_ctx::turn_settled();
            observe_script_hook(bus, ty, phase, ctx).await;
            let (ty, phase, ctx) = super::script_hook_ctx::turn_end();
            observe_script_hook(bus, ty, phase, ctx).await;
        }
        if let Some(span) = agent_turn_span {
            use super::obs::StepEndReason;
            span.finish(if abort_reason.is_some() {
                StepEndReason::Aborted
            } else {
                StepEndReason::Ok
            });
        }
        yield XyEvent::AgentEnd {
            messages: history,
            reason: abort_reason,
        };
    }
}

#[cfg(test)]
mod loop_middleware_tests {
    use std::sync::Arc;

    use futures::StreamExt;
    use xylitol_plugin_sdk::{
        BeforeStepCtx, BeforeToolCtx, DecideContinueCtx, LoopMiddleware, LoopMiddlewareRegistry,
        LoopStage, LoopStep,
    };

    use crate::agent::capabilities::QueueMode;
    use crate::agent::model::registry::ModelRegistry;
    use crate::agent::runtime::baseline::RuntimeBaseline;
    use crate::agent::tools::ToolSet;
    use crate::test_support::{MemoryStore, NoopSink, dummy_model_builder};
    use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind, XyModelMeta};

    use super::{AgentEndReason, note_abort};

    struct DenyBash;

    #[async_trait::async_trait]
    impl LoopMiddleware for DenyBash {
        fn id(&self) -> &str {
            "deny-bash"
        }

        fn stages(&self) -> &'static [LoopStage] {
            &[LoopStage::BeforeTool]
        }

        async fn before_tool(&self, ctx: BeforeToolCtx) -> LoopStep<BeforeToolCtx> {
            if ctx.tool_name == "bash" {
                LoopStep::stopped(ctx)
            } else {
                LoopStep::continued(ctx)
            }
        }
    }

    struct StopAtSettle;

    #[async_trait::async_trait]
    impl LoopMiddleware for StopAtSettle {
        fn id(&self) -> &str {
            "stop-at-settle"
        }

        fn stages(&self) -> &'static [LoopStage] {
            &[LoopStage::DecideContinue]
        }

        async fn decide_continue(&self, ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
            LoopStep::stopped(ctx)
        }
    }

    fn registry() -> Arc<LoopMiddlewareRegistry> {
        Arc::new(LoopMiddlewareRegistry::new())
    }

    fn baseline() -> RuntimeBaseline {
        let mut reg = ModelRegistry::new();
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: XyModelConfig {
                kind: XyModelKind::Fake,
                api_key: String::new(),
                model: "mock".into(),
                base_url: None,
                api: None,
                compat: None,
                prompt_cache: None,
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
        let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> = Arc::new(MemoryStore::new());
        let sink: Arc<dyn xylitol_plugin_sdk::XyEventSink> = Arc::new(NoopSink);
        RuntimeBaseline {
            store,
            sink,
            permission: crate::permission::allow_all_permission(),
            hook_bus: None,
            model_registry: reg,
            model_builder: dummy_model_builder(),
            cwd: ".".into(),
            steering_mode: QueueMode::default(),
            follow_up_mode: QueueMode::default(),
            compaction_settings: None,
            compaction_factory: crate::agent::compaction::noop_compaction_factory(),
            tools: ToolSet::empty(),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            skills: Vec::new(),
            batch_mode: xylitol_plugin_sdk::XyBatchMode::Sequential,
        }
    }

    #[test]
    fn abort_reason_first_non_ok_wins() {
        let mut slot = None;
        note_abort(AgentEndReason::MiddlewareFault, &mut slot);
        note_abort(AgentEndReason::UserCancel, &mut slot);
        assert_eq!(slot, Some(AgentEndReason::MiddlewareFault));
    }

    struct OneShotText;

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyModel for OneShotText {
        fn name(&self) -> &str {
            "one-shot-text"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
            _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
            _stream: bool,
            _options: xylitol_plugin_sdk::generate::XyGenerateOptions,
        ) -> Result<xylitol_plugin_sdk::XyStream, xylitol_plugin_sdk::error::XyError> {
            Ok(Box::pin(futures::stream::iter(vec![
                Ok(xylitol_plugin_sdk::model::XyChunk::TextDelta("done".into())),
                Ok(xylitol_plugin_sdk::model::XyChunk::Done {
                    finish_reason: xylitol_plugin_sdk::message::XyStopReason::Stop,
                    usage: None,
                }),
            ])))
        }
    }

    struct RetryOnceText {
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyModel for RetryOnceText {
        fn name(&self) -> &str {
            "retry-once-text"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
            _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
            _stream: bool,
            _options: xylitol_plugin_sdk::generate::XyGenerateOptions,
        ) -> Result<xylitol_plugin_sdk::XyStream, xylitol_plugin_sdk::error::XyError> {
            use std::sync::atomic::Ordering;
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err(xylitol_plugin_sdk::error::XyError::provider(
                    xylitol_plugin_sdk::error::XyProviderError {
                        message: "rate limit exceeded".into(),
                        ..Default::default()
                    },
                ));
            }
            Ok(Box::pin(futures::stream::iter(vec![
                Ok(xylitol_plugin_sdk::model::XyChunk::TextDelta("ok".into())),
                Ok(xylitol_plugin_sdk::model::XyChunk::Done {
                    finish_reason: xylitol_plugin_sdk::message::XyStopReason::Stop,
                    usage: None,
                }),
            ])))
        }
    }

    #[tokio::test]
    async fn auto_retry_end_only_after_real_retry_in_the_same_step() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_for_builder = calls.clone();
        let mut base = baseline();
        base.model_builder = Arc::new(move |_| {
            Arc::new(RetryOnceText {
                calls: calls_for_builder.clone(),
            }) as Arc<dyn xylitol_plugin_sdk::XyModel>
        });
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-retry").expect("bind");
        runtime.follow_up("second step");

        let mut events = Vec::new();
        let mut stream = runtime.submit_root("go").await;
        while let Some(ev) = stream.next().await {
            events.push(ev);
        }

        let starts = events
            .iter()
            .filter(|e| matches!(e, crate::agent::runtime::XyEvent::AutoRetryStart { .. }))
            .count();
        let ends: Vec<(bool, u32)> = events
            .iter()
            .filter_map(|e| match e {
                crate::agent::runtime::XyEvent::AutoRetryEnd { success, attempt } => {
                    Some((*success, *attempt))
                }
                _ => None,
            })
            .collect();
        assert_eq!(starts, 1, "one retry expected: {events:?}");
        assert_eq!(
            ends,
            vec![(true, 1)],
            "only the retried step may emit AutoRetryEnd: {events:?}"
        );
        let steps = events
            .iter()
            .filter(|e| matches!(e, crate::agent::runtime::XyEvent::StepStart { .. }))
            .count();
        assert_eq!(
            steps, 2,
            "the second step must run without a retry: {events:?}"
        );
    }

    fn user_texts(events: &[crate::agent::runtime::XyEvent]) -> Vec<String> {
        events
            .iter()
            .rev()
            .find_map(|event| match event {
                crate::agent::runtime::XyEvent::AgentEnd { messages, .. } => Some(
                    messages
                        .iter()
                        .filter(|message| message.role_name() == "user")
                        .map(|message| message.text())
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .expect("final AgentEnd must carry history")
    }

    #[tokio::test]
    async fn follow_up_fifo_opens_one_turn_per_submission() {
        let mut base = baseline();
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-fifo").expect("bind");
        runtime.follow_up("f1");
        runtime.follow_up("f2");
        runtime.follow_up("f3");

        let mut stream = runtime.submit_root("p").await;
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        let boundaries: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                crate::agent::runtime::XyEvent::AgentStart { .. } => Some("start"),
                crate::agent::runtime::XyEvent::AgentEnd { .. } => Some("end"),
                _ => None,
            })
            .collect();
        assert_eq!(
            boundaries,
            vec![
                "start", "end", "start", "end", "start", "end", "start", "end"
            ],
            "each submission must open its own turn: {events:?}"
        );
        assert_eq!(
            user_texts(&events),
            vec!["p", "f1", "f2", "f3"],
            "follow-ups must be delivered one by one in FIFO order"
        );
    }

    #[tokio::test]
    async fn steer_after_settle_becomes_own_turn_via_fifo() {
        let mut base = baseline();
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-steer-late").expect("bind");

        let mut stream = runtime.submit_root("first").await;
        while stream.next().await.is_some() {}
        assert_eq!(runtime.queue_stats().steer_count, 0, "steer drained");

        runtime.steer("late steer");
        assert_eq!(
            runtime.queue_stats().follow_up_count,
            1,
            "a steer arriving with no turn in flight must land in the follow_up FIFO"
        );
        assert_eq!(runtime.queue_stats().steer_count, 0);

        let mut stream = runtime.submit_root("second").await;
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        let boundaries: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                crate::agent::runtime::XyEvent::AgentStart { .. } => Some("start"),
                crate::agent::runtime::XyEvent::AgentEnd { .. } => Some("end"),
                _ => None,
            })
            .collect();
        assert_eq!(
            boundaries,
            vec!["start", "end", "start", "end"],
            "the settled steer must open its own turn with independent boundaries: {events:?}"
        );
        let steps = events
            .iter()
            .filter(|e| matches!(e, crate::agent::runtime::XyEvent::StepStart { .. }))
            .count();
        assert_eq!(
            steps, 2,
            "the steer must not merge into the second submission's model call: {events:?}"
        );
        let delivered_user_texts: Vec<String> = events
            .iter()
            .filter_map(|event| match event {
                crate::agent::runtime::XyEvent::MessageStart {
                    role,
                    message: Some(message),
                    ..
                } if role == "user" => Some(message.text()),
                _ => None,
            })
            .collect();
        assert_eq!(
            delivered_user_texts,
            vec!["late steer"],
            "the settled steer must be delivered as a fresh user message in its own turn"
        );
    }

    use crate::agent::compaction::{CompactionError, CompactionSettings, XyCompaction};
    use xylitol_plugin_sdk::loop_middleware::{CompactRequest, LoopControl};
    use xylitol_plugin_sdk::{
        CompactionSummaryBinding, EstimateOpts, FixedRequestContext, OverflowCompactOutcome,
        XyEventSink,
    };

    struct CompactAtSettle;

    #[async_trait::async_trait]
    impl LoopMiddleware for CompactAtSettle {
        fn id(&self) -> &str {
            "compact-at-settle"
        }

        fn stages(&self) -> &'static [LoopStage] {
            &[LoopStage::DecideContinue]
        }

        async fn decide_continue(&self, mut ctx: DecideContinueCtx) -> LoopStep<DecideContinueCtx> {
            if ctx.settle_count == 0 {
                ctx.compact = Some(CompactRequest::default());
                LoopStep {
                    ctx,
                    control: LoopControl::Compact,
                    properties: Vec::new(),
                }
            } else {
                LoopStep::stopped(ctx)
            }
        }
    }

    struct FailingCompaction {
        settings: CompactionSettings,
    }

    #[async_trait::async_trait]
    impl XyCompaction for FailingCompaction {
        fn settings(&self) -> &CompactionSettings {
            &self.settings
        }

        async fn compact(
            &self,
            _store: &dyn xylitol_plugin_sdk::XySessionStore,
            _sid: &str,
            _summary: &CompactionSummaryBinding,
            _event_sink: &dyn XyEventSink,
            _instructions: Option<String>,
            _reason: &str,
            _context_window: u64,
            _fixed_context: Option<&FixedRequestContext>,
            _fallback_notice_emitted: &mut bool,
        ) -> Result<(), CompactionError> {
            Err("compaction boom".into())
        }

        async fn maybe_overflow_compact(
            &self,
            _store: &dyn xylitol_plugin_sdk::XySessionStore,
            _sid: &str,
            _summary: &CompactionSummaryBinding,
            _event_sink: &dyn XyEventSink,
            _context_window: u64,
            _last_assistant: &xylitol_plugin_sdk::message::AgentMessage,
            _current_provider: &str,
            _current_model_id: &str,
            _overflow_recovery_attempted: bool,
            _fixed_context: Option<&FixedRequestContext>,
            _turn_obs_parent: Option<fastrace::prelude::SpanContext>,
            _fallback_notice_emitted: &mut bool,
            _floor_notice_emitted: Option<&mut bool>,
        ) -> Result<OverflowCompactOutcome, CompactionError> {
            Ok(OverflowCompactOutcome::Skipped)
        }

        async fn maybe_auto_compact(
            &self,
            _store: &dyn xylitol_plugin_sdk::XySessionStore,
            _sid: &str,
            _summary: &CompactionSummaryBinding,
            _event_sink: &dyn XyEventSink,
            _context_window: u64,
            _estimate_opts: &EstimateOpts,
            _last_assistant: Option<&xylitol_plugin_sdk::message::AgentMessage>,
            _precomputed: Option<&xylitol_plugin_sdk::ContextTokenEstimate>,
            _fixed_context: Option<&FixedRequestContext>,
            _turn_obs_parent: Option<fastrace::prelude::SpanContext>,
            _fallback_notice_emitted: &mut bool,
            _floor_notice_emitted: Option<&mut bool>,
        ) -> Result<bool, CompactionError> {
            Ok(false)
        }
    }

    struct RecordingText {
        calls: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyModel for RecordingText {
        fn name(&self) -> &str {
            "recording-text"
        }

        async fn generate_stream(
            &self,
            messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
            _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
            _stream: bool,
            _options: xylitol_plugin_sdk::generate::XyGenerateOptions,
        ) -> Result<xylitol_plugin_sdk::XyStream, xylitol_plugin_sdk::error::XyError> {
            self.calls.lock().unwrap().push(format!("{messages:?}"));
            Ok(Box::pin(futures::stream::iter(vec![
                Ok(xylitol_plugin_sdk::model::XyChunk::TextDelta("done".into())),
                Ok(xylitol_plugin_sdk::model::XyChunk::Done {
                    finish_reason: xylitol_plugin_sdk::message::XyStopReason::Stop,
                    usage: None,
                }),
            ])))
        }
    }

    #[tokio::test]
    async fn decide_continue_stop_requeues_settled_steer_into_follow_up_fifo() {
        let mut base = baseline();
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let mut runtime = base.materialize_runtime();
        let chain = registry();
        chain.register(Arc::new(StopAtSettle)).expect("register");
        runtime.set_loop_middleware(Some(chain));
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-stop").expect("bind");

        let mut stream = runtime.submit_root("p").await;
        let mut events = Vec::new();
        let mut steered = false;
        while let Some(event) = stream.next().await {
            if !steered
                && matches!(
                    &event,
                    crate::agent::runtime::XyEvent::MessageEnd { role, .. } if role == "assistant"
                )
            {
                runtime.steer("mid settle");
                steered = true;
            }
            events.push(event);
        }
        assert!(
            steered,
            "test must steer before the settle drain: {events:?}"
        );
        assert_eq!(
            runtime.queue_stats().follow_up_count,
            1,
            "middleware stop must requeue the drained steer into the follow_up FIFO: {events:?}"
        );
        assert_eq!(runtime.queue_stats().steer_count, 0);
    }

    #[tokio::test]
    async fn middleware_compact_success_feeds_settled_steer_to_next_step() {
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut base = baseline();
        base.model_builder = Arc::new({
            let calls = Arc::clone(&calls);
            move |_| {
                let calls = Arc::clone(&calls);
                Arc::new(RecordingText { calls }) as Arc<dyn xylitol_plugin_sdk::XyModel>
            }
        });
        let mut runtime = base.materialize_runtime();
        let chain = registry();
        chain.register(Arc::new(CompactAtSettle)).expect("register");
        runtime.set_loop_middleware(Some(chain));
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-compact").expect("bind");

        let mut stream = runtime.submit_root("p").await;
        let mut steered = false;
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            if !steered
                && matches!(
                    &event,
                    crate::agent::runtime::XyEvent::MessageEnd { role, .. } if role == "assistant"
                )
            {
                runtime.steer("mid steer");
                steered = true;
            }
            events.push(event);
        }
        assert!(steered);
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            2,
            "compact must continue the loop into a second step (settle_count must advance after compact): {events:?}"
        );
        assert!(
            calls[1].contains("mid steer"),
            "the drained steer must reach the model call after compaction: {}",
            calls[1]
        );
        let step_indices: Vec<u32> = events
            .iter()
            .filter_map(|event| match event {
                crate::agent::runtime::XyEvent::StepStart { step_index, .. } => Some(*step_index),
                _ => None,
            })
            .collect();
        assert_eq!(
            step_indices,
            vec![0, 1],
            "the post-compact step must advance step_index instead of repeating it"
        );
    }

    #[tokio::test]
    async fn middleware_compact_failure_requeues_settled_steer() {
        let mut base = baseline();
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        base.compaction_factory =
            Arc::new(|settings| Arc::new(FailingCompaction { settings }) as Arc<dyn XyCompaction>);
        let mut runtime = base.materialize_runtime();
        let chain = registry();
        chain.register(Arc::new(CompactAtSettle)).expect("register");
        runtime.set_loop_middleware(Some(chain));
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-compact-fail").expect("bind");

        let mut stream = runtime.submit_root("p").await;
        let mut events = Vec::new();
        let mut steered = false;
        while let Some(event) = stream.next().await {
            if !steered
                && matches!(
                    &event,
                    crate::agent::runtime::XyEvent::MessageEnd { role, .. } if role == "assistant"
                )
            {
                runtime.steer("mid steer");
                steered = true;
            }
            events.push(event);
        }
        assert!(steered);
        assert_eq!(
            runtime.queue_stats().follow_up_count,
            1,
            "failed middleware compact must requeue the drained steer: {events:?}"
        );
        assert!(events.iter().any(|event| {
            matches!(event, crate::agent::runtime::XyEvent::Error(e) if e.message.contains("compact failed"))
        }));
    }

    #[tokio::test]
    async fn follow_up_all_mode_still_opens_one_turn_per_submission() {
        let mut base = baseline();
        base.follow_up_mode = QueueMode::All;
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-all").expect("bind");
        runtime.follow_up("f1");
        runtime.follow_up("f2");

        let mut stream = runtime.submit_root("p").await;
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        let boundaries: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                crate::agent::runtime::XyEvent::AgentStart { .. } => Some("start"),
                crate::agent::runtime::XyEvent::AgentEnd { .. } => Some("end"),
                _ => None,
            })
            .collect();
        assert_eq!(
            boundaries,
            vec!["start", "end", "start", "end", "start", "end"],
            "All mode must still open one turn per follow_up submission: {events:?}"
        );
        assert_eq!(
            user_texts(&events),
            vec!["p", "f1", "f2"],
            "no follow_up submission may be dropped in All mode"
        );
    }

    #[tokio::test]
    async fn abort_mid_turn_clears_steer_and_pops_follow_up_into_new_turns() {
        let mut base = baseline();
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-abort-fifo").expect("bind");
        runtime.follow_up("f1");
        runtime.follow_up("f2");

        let mut stream = runtime.submit_root("p").await;
        let mut events = Vec::new();
        let first = stream.next().await.expect("AgentStart");
        assert!(matches!(
            first,
            crate::agent::runtime::XyEvent::AgentStart { .. }
        ));
        events.push(first);
        runtime.steer("to-clear");
        assert_eq!(
            runtime.queue_stats().steer_count,
            1,
            "steer while the turn is in flight must queue for the current turn"
        );
        runtime.abort();
        assert_eq!(runtime.queue_stats().steer_count, 0, "abort clears steer");
        assert_eq!(
            runtime.queue_stats().follow_up_count,
            2,
            "abort keeps the follow_up FIFO"
        );
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        let boundaries: Vec<(String, Option<AgentEndReason>)> = events
            .iter()
            .filter_map(|event| match event {
                crate::agent::runtime::XyEvent::AgentStart { .. } => {
                    Some(("start".to_string(), None))
                }
                crate::agent::runtime::XyEvent::AgentEnd { reason, .. } => {
                    Some(("end".to_string(), *reason))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            boundaries,
            vec![
                ("start".to_string(), None),
                ("end".to_string(), Some(AgentEndReason::UserCancel)),
                ("start".to_string(), None),
                ("end".to_string(), None),
                ("start".to_string(), None),
                ("end".to_string(), None),
            ],
            "abort must close the current turn then immediately open turns for f1 and f2: {events:?}"
        );
        assert!(
            events.iter().any(
                |e| matches!(e, crate::agent::runtime::XyEvent::Error(err) if err.is_aborted())
            ),
            "the aborted turn must surface aborted: {events:?}"
        );
        assert_eq!(
            user_texts(&events),
            vec!["p", "f1", "f2"],
            "follow-ups must be delivered in FIFO order after abort: {events:?}"
        );
        assert!(
            !user_texts(&events).contains(&"to-clear".to_string()),
            "steer cleared by abort must never reach history: {events:?}"
        );
        let stats = runtime.queue_stats();
        assert_eq!(stats.steer_count, 0);
        assert_eq!(stats.follow_up_count, 0);
    }

    struct RoundPlanModel {
        rounds: std::sync::Mutex<Vec<Vec<xylitol_plugin_sdk::model::XyChunk>>>,
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl RoundPlanModel {
        fn round_index(&self) -> usize {
            self.call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyModel for RoundPlanModel {
        fn name(&self) -> &str {
            "round-plan-mock"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
            _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
            _stream: bool,
            _options: xylitol_plugin_sdk::generate::XyGenerateOptions,
        ) -> Result<xylitol_plugin_sdk::XyStream, xylitol_plugin_sdk::error::XyError> {
            let index = self.round_index();
            let chunks = {
                let rounds = self.rounds.lock().expect("rounds");
                rounds.get(index).cloned().ok_or_else(|| {
                    xylitol_plugin_sdk::error::XyError::provider(
                        xylitol_plugin_sdk::error::XyProviderError {
                            message: format!("round plan exhausted at {index}"),
                            ..Default::default()
                        },
                    )
                })?
            };
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }

    struct GateTool {
        started: tokio::sync::mpsc::UnboundedSender<()>,
        release: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyTool for GateTool {
        fn name(&self) -> &str {
            "gate_tool"
        }

        fn description(&self) -> &str {
            "blocks until released"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}})
        }

        async fn execute_outcome(
            &self,
            _ctx: &xylitol_plugin_sdk::XyToolCtx,
            _args: serde_json::Value,
        ) -> Result<xylitol_plugin_sdk::XyToolOutcome, xylitol_plugin_sdk::error::XyToolError>
        {
            let _ = self.started.send(());
            self.release.notified().await;
            Ok(xylitol_plugin_sdk::XyToolOutcome::text("gate ok"))
        }
    }

    fn tool_call_round(name: &str) -> Vec<xylitol_plugin_sdk::model::XyChunk> {
        use xylitol_plugin_sdk::message::XyStopReason;
        use xylitol_plugin_sdk::model::XyChunk;
        vec![
            XyChunk::ToolCallStart {
                id: "call-1".into(),
                name: name.into(),
            },
            XyChunk::ToolCallEnd {
                id: "call-1".into(),
                name: name.into(),
                args: serde_json::json!({}),
            },
            XyChunk::Done {
                finish_reason: XyStopReason::Stop,
                usage: None,
            },
        ]
    }

    fn text_round(text: &str) -> Vec<xylitol_plugin_sdk::model::XyChunk> {
        use xylitol_plugin_sdk::message::XyStopReason;
        use xylitol_plugin_sdk::model::XyChunk;
        vec![
            XyChunk::TextDelta(text.into()),
            XyChunk::Done {
                finish_reason: XyStopReason::Stop,
                usage: None,
            },
        ]
    }

    #[tokio::test]
    async fn steer_mid_turn_injects_into_following_step_without_new_agent_start() {
        use crate::agent::tools::ToolSet;

        let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
        let release = Arc::new(tokio::sync::Notify::new());
        let mut base = baseline();
        base.tools = ToolSet::from_iter(vec![Arc::new(GateTool {
            started: started_tx,
            release: release.clone(),
        }) as Arc<dyn xylitol_plugin_sdk::XyTool>]);
        base.model_builder = Arc::new(move |_| {
            Arc::new(RoundPlanModel {
                rounds: std::sync::Mutex::new(vec![
                    tool_call_round("gate_tool"),
                    text_round("settled once"),
                    text_round("after steer"),
                ]),
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }) as Arc<dyn xylitol_plugin_sdk::XyModel>
        });
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-steer-midturn").expect("bind");

        let mut stream = runtime.submit_root("go").await;
        let mut events = Vec::new();
        let mut saw_tool_start = false;
        while let Some(event) = stream.next().await {
            saw_tool_start |= matches!(
                event,
                crate::agent::runtime::XyEvent::ToolExecutionStart { .. }
            );
            events.push(event);
            if saw_tool_start {
                break;
            }
        }
        assert!(
            saw_tool_start,
            "tool must start before steering: {events:?}"
        );

        tokio::time::timeout(std::time::Duration::from_secs(2), started_rx.recv())
            .await
            .expect("gate tool entered")
            .expect("gate tool signal");
        runtime.steer("mid-turn steer");
        assert_eq!(
            runtime.queue_stats().steer_count,
            1,
            "steer while the turn is in flight must queue for the current turn"
        );
        release.notify_waiters();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        let boundaries: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                crate::agent::runtime::XyEvent::AgentStart { .. } => Some("start"),
                crate::agent::runtime::XyEvent::AgentEnd { .. } => Some("end"),
                _ => None,
            })
            .collect();
        assert_eq!(
            boundaries,
            vec!["start", "end"],
            "mid-turn steer must stay inside the current turn boundaries: {events:?}"
        );
        let steps = events
            .iter()
            .filter(|e| matches!(e, crate::agent::runtime::XyEvent::StepStart { .. }))
            .count();
        assert_eq!(
            steps, 3,
            "tool step, settled step, and post-steer step in one turn: {events:?}"
        );
        let tool_start_at = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    crate::agent::runtime::XyEvent::ToolExecutionStart { .. }
                )
            })
            .expect("tool start position");
        let steer_injected_at = events
            .iter()
            .position(|event| match event {
                crate::agent::runtime::XyEvent::MessageStart {
                    role,
                    message: Some(message),
                    ..
                } => role == "user" && message.text() == "mid-turn steer",
                _ => false,
            })
            .expect("steer must be injected as a user message");
        assert!(
            steer_injected_at > tool_start_at,
            "steer must be injected in a later step of the same turn"
        );
        assert_eq!(runtime.queue_stats().steer_count, 0);
        let history = events
            .iter()
            .rev()
            .find_map(|event| match event {
                crate::agent::runtime::XyEvent::AgentEnd { messages, .. } => Some(messages.clone()),
                _ => None,
            })
            .expect("AgentEnd history");
        let user_history: Vec<String> = history
            .iter()
            .filter(|m| m.role_name() == "user")
            .map(|m| m.text())
            .collect();
        assert_eq!(
            user_history,
            vec!["go", "mid-turn steer"],
            "steer rides the same turn right after the tool step: {user_history:?}"
        );
    }

    struct GoalJsonText;

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyModel for GoalJsonText {
        fn name(&self) -> &str {
            "goal-json-text"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
            _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
            _stream: bool,
            _options: xylitol_plugin_sdk::generate::XyGenerateOptions,
        ) -> Result<xylitol_plugin_sdk::XyStream, xylitol_plugin_sdk::error::XyError> {
            Ok(Box::pin(futures::stream::iter(vec![
                Ok(xylitol_plugin_sdk::model::XyChunk::TextDelta(
                    r#"{"verdict":"not_met","evidence":"still failing"}"#.into(),
                )),
                Ok(xylitol_plugin_sdk::model::XyChunk::Done {
                    finish_reason: xylitol_plugin_sdk::message::XyStopReason::Stop,
                    usage: None,
                }),
            ])))
        }
    }

    fn goal_defined(max_iterations: u64) -> xylitol_plugin_sdk::session::SessionEntry {
        use xylitol_plugin_sdk::session::{CUSTOM_TYPE_AUTOMATION, CustomEntry, EntryBase};
        xylitol_plugin_sdk::session::SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AUTOMATION.into(),
            data: serde_json::json!({
                "kind": "goal_defined",
                "goal_id": "g1",
                "branch_id": "sess-goal",
                "objective": "tests pass",
                "acceptance": "cargo test exits 0",
                "max_iterations": max_iterations,
                "created_at": 0
            }),
        })
    }

    #[tokio::test]
    async fn goal_continue_delivers_guidance_as_new_turn() {
        let mut base = baseline();
        base.model_builder =
            Arc::new(|_| Arc::new(GoalJsonText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let store = base.store.clone();
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-goal").expect("bind");
        store
            .append_session_entry("sess-goal", &goal_defined(2))
            .await
            .expect("append goal");

        let mut stream = runtime.submit_root("go").await;
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        let boundaries: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                crate::agent::runtime::XyEvent::AgentStart { .. } => Some("start"),
                crate::agent::runtime::XyEvent::AgentEnd { .. } => Some("end"),
                _ => None,
            })
            .collect();
        assert_eq!(
            boundaries,
            vec!["start", "end", "start", "end"],
            "goal continuation must open its own turn: {events:?}"
        );
        let texts = user_texts(&events);
        assert_eq!(
            texts.len(),
            2,
            "one continuation submission expected: {texts:?}"
        );
        assert_eq!(texts[0], "go");
        assert!(
            texts[1].contains("still failing"),
            "evaluator guidance must ride the next submission: {texts:?}"
        );
        assert!(
            texts[1].starts_with("<system-reminder>") && texts[1].ends_with("</system-reminder>"),
            "goal continuation must carry the source marker: {texts:?}"
        );
        let goals = xylitol_plugin_sdk::session::project_goals(
            &store.load_entries("sess-goal").await.expect("load entries"),
            "sess-goal",
        );
        assert_eq!(
            goals[0].state,
            xylitol_plugin_sdk::session::GoalState::Blocked,
            "second not_met must hit max_iterations and block"
        );
    }

    struct CaptureBeforeStep {
        seen: Arc<std::sync::Mutex<Vec<(u32, bool)>>>,
    }

    #[async_trait::async_trait]
    impl LoopMiddleware for CaptureBeforeStep {
        fn id(&self) -> &str {
            "capture-before-step"
        }

        fn stages(&self) -> &'static [LoopStage] {
            &[LoopStage::BeforeStep]
        }

        async fn before_step(&self, ctx: BeforeStepCtx) -> LoopStep<BeforeStepCtx> {
            self.seen
                .lock()
                .expect("seen")
                .push((ctx.step_index, ctx.has_new_user_messages));
            LoopStep::continued(ctx)
        }
    }

    #[tokio::test]
    async fn before_step_flags_new_user_input_at_turn_boundaries() {
        let reg = registry();
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Arc::new(CaptureBeforeStep { seen: seen.clone() }))
            .expect("register");
        let mut base = baseline();
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-before-step").expect("bind");
        runtime.set_loop_middleware(Some(reg));
        runtime.follow_up("second turn");
        let mut stream = runtime.submit_root("go").await;
        while stream.next().await.is_some() {}
        let seen = seen.lock().expect("seen");
        assert_eq!(
            *seen,
            vec![(0, true), (0, true)],
            "each turn starts at step 0 and carries new user input"
        );
    }

    struct MarkerTurnInjector {
        store: Arc<dyn xylitol_plugin_sdk::XySessionStore>,
        session_id: String,
    }

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::XyTurnInjector for MarkerTurnInjector {
        fn id(&self) -> &str {
            "marker-turn-injector"
        }

        async fn on_turn_submission(
            &self,
            _ctx: &BeforeStepCtx,
        ) -> Vec<xylitol_plugin_sdk::message::AgentMessage> {
            use xylitol_plugin_sdk::message::{AgentMessage, EnvMessage};
            use xylitol_plugin_sdk::session::{CustomMessageEntry, EntryBase, SessionEntry};
            let content = serde_json::json!("<system-reminder>reminder</system-reminder>");
            let entry = SessionEntry::CustomMessage(CustomMessageEntry {
                base: EntryBase {
                    entry_type: "customMessage".into(),
                    id: String::new(),
                    parent_id: None,
                    timestamp: 0,
                },
                custom_type: "context_reminder".into(),
                content: content.clone(),
                display: true,
                details: None,
            });
            let _ = self
                .store
                .append_session_entry(&self.session_id, &entry)
                .await;
            vec![AgentMessage::Env(EnvMessage::CustomMessage {
                custom_type: "context_reminder".into(),
                content,
                display: serde_json::json!(true),
                details: serde_json::Value::Null,
            })]
        }
    }

    #[tokio::test]
    async fn turn_injector_runs_before_user_message_persist() {
        use xylitol_plugin_sdk::session::SessionEntry;
        let reg = registry();
        let mut base = baseline();
        let store = base.store.clone();
        reg.register_turn_injector(Arc::new(MarkerTurnInjector {
            store: store.clone(),
            session_id: "sess-inject".into(),
        }))
        .expect("register turn injector");
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-inject").expect("bind");
        runtime.set_loop_middleware(Some(reg));
        let mut stream = runtime.submit_root("go").await;
        while stream.next().await.is_some() {}
        let entries = store.load_entries("sess-inject").await.expect("entries");
        let transcript: Vec<&SessionEntry> = entries
            .iter()
            .filter(|entry| entry.anchors_transcript())
            .collect();
        let reminder = transcript
            .iter()
            .position(|entry| {
                matches!(
                    entry,
                    SessionEntry::CustomMessage(custom)
                        if custom.custom_type == "context_reminder"
                )
            })
            .expect("reminder row must be persisted");
        let user = transcript
            .iter()
            .position(|entry| {
                matches!(
                    entry,
                    SessionEntry::Message(message)
                        if message.message.get("role").and_then(|role| role.as_str()) == Some("user")
                )
            })
            .expect("user message must be persisted");
        assert!(
            reminder < user,
            "reminder row must precede the user message: {transcript:?}"
        );
    }

    #[tokio::test]
    async fn step_limit_stops_after_completed_step_without_abort() {
        use xylitol_plugin_sdk::message::{AgentMessage, XyLlmMessage, XyStopReason};

        let mut base = baseline();
        base.model_builder =
            Arc::new(|_| Arc::new(OneShotText) as Arc<dyn xylitol_plugin_sdk::XyModel>);
        let store = base.store.clone();
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-step-limit").expect("bind");
        runtime.set_should_stop_after_step(Some(crate::agent::max_steps_stop_hook(1)));

        let mut events = Vec::new();
        let mut stream = runtime.submit_root("go").await;
        while let Some(ev) = stream.next().await {
            events.push(ev);
        }

        let step_starts = events
            .iter()
            .filter(|e| matches!(e, crate::agent::runtime::XyEvent::StepStart { .. }))
            .count();
        assert_eq!(
            step_starts, 1,
            "step limit must stop the first step: {events:?}"
        );
        let messages = events
            .iter()
            .find_map(|e| match e {
                crate::agent::runtime::XyEvent::AgentEnd { messages, .. } => Some(messages),
                _ => None,
            })
            .expect("AgentEnd must be emitted");
        let assistant = messages
            .iter()
            .rev()
            .find(|m| m.role_name() == "assistant")
            .expect("completed assistant message must not be aborted");
        match assistant {
            AgentMessage::Llm(XyLlmMessage::AssistantMessage { stop_reason, .. }) => {
                assert_eq!(*stop_reason, Some(XyStopReason::Stop));
            }
            other => panic!("unexpected assistant shape: {other:?}"),
        }
        let entries = store
            .load_entries("sess-step-limit")
            .await
            .expect("persisted session");
        assert!(
            entries.iter().any(|e| matches!(
                e.as_agent_message(),
                Some(AgentMessage::Llm(XyLlmMessage::AssistantMessage { .. }))
            )),
            "completed step must stay persisted"
        );
    }

    #[tokio::test]
    async fn before_tool_stop_blocks_execution_and_ends_run() {
        let reg = registry();
        reg.register(Arc::new(DenyBash)).expect("register");
        let mut runtime = baseline().materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-deny").expect("bind");
        runtime.set_loop_middleware(Some(reg));
        assert_eq!(runtime.loop_middleware_ids(), vec!["deny-bash".to_string()]);

        let mut events = Vec::new();
        let mut stream = runtime.submit_root("go").await;
        while let Some(ev) = stream.next().await {
            events.push(ev);
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, crate::agent::runtime::XyEvent::AgentEnd { .. })),
            "run must end: {events:?}"
        );
    }

    #[tokio::test]
    async fn decide_continue_stop_ends_run_without_extra_turn() {
        let reg = registry();
        reg.register(Arc::new(StopAtSettle)).expect("register");
        let mut runtime = baseline().materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-stop").expect("bind");
        runtime.set_loop_middleware(Some(reg));
        let mut events = Vec::new();
        let mut stream = runtime.submit_root("go").await;
        while let Some(ev) = stream.next().await {
            events.push(ev);
        }
        let turns = events
            .iter()
            .filter(|e| matches!(e, crate::agent::runtime::XyEvent::StepStart { .. }))
            .count();
        assert_eq!(turns, 1, "stop must not start another turn: {events:?}");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, crate::agent::runtime::XyEvent::AgentEnd { .. })),
            "run must end: {events:?}"
        );
    }

    #[tokio::test]
    async fn message_is_persisted_before_message_end_is_emitted() {
        let base = baseline();
        let store = base.store.clone();
        let mut runtime = base.materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-persist").expect("bind");

        let mut stream = runtime.submit_root("hi").await;
        let mut seen_end = false;
        while let Some(ev) = stream.next().await {
            if matches!(ev, crate::agent::runtime::XyEvent::MessageEnd { .. }) {
                seen_end = true;
                let entries = store
                    .load_entries("sess-persist")
                    .await
                    .expect("session must exist by MessageEnd");
                assert!(
                    !entries.is_empty(),
                    "the transcript must already be on disk when MessageEnd arrives"
                );
                break;
            }
        }
        assert!(seen_end, "the run must emit MessageEnd");
    }

    #[tokio::test]
    async fn message_update_carries_only_the_message_snapshot() {
        let ev = crate::agent::runtime::react::assistant::streaming_message_update(
            "hello",
            "think",
            None,
            &[],
        );
        let value = serde_json::to_value(&ev).expect("serialize");
        assert!(
            value.get("text").is_none() && value.get("thinking").is_none(),
            "MessageUpdate must not carry duplicated text/thinking: {value}"
        );
        assert!(
            value.get("message").is_some(),
            "the message snapshot is the single source of truth: {value}"
        );
    }

    #[tokio::test]
    async fn image_parts_without_declared_modality_are_rejected() {
        let mut runtime = baseline().materialize_runtime();
        runtime.select_model("mock").await.expect("select");
        runtime.bind_session("sess-image").expect("bind");

        let mut events = Vec::new();
        let mut stream = runtime
            .submit_parts(vec![xylitol_plugin_sdk::message::AgentPart::image(
                "image/png",
                "abc",
            )])
            .await;
        while let Some(ev) = stream.next().await {
            events.push(ev);
        }

        let rejection = events.iter().find_map(|event| match event {
            crate::agent::runtime::XyEvent::Error(error)
                if error.message.contains("does not declare image input") =>
            {
                Some(error.message.as_str())
            }
            _ => None,
        });
        assert!(
            rejection.is_some(),
            "image input must be rejected when the model declares text only: {events:?}"
        );
    }
}
