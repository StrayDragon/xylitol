//! Agent execution loop — core ReAct loop with full event stream, hooks, and tool batch modes.
//!
//! `async_stream` keeps the turn loop as one yield-capable block; helpers below extract
//! repeated MessageUpdate / turn-end / assistant assembly without a sub-turn state machine.
//! Soft ~1200 / hard ~2000 LOC (see `src/AGENTS.md`); further split only when edit pain
//! forces a state machine.
//!
//! Tool batch scheduling lives in `tool_batch` + `tool_exec` (c1545). Product default is
//! Sequential (source-order await); BarrierParallel fans out ParallelSafe windows.

mod assistant;
pub(crate) mod support;
#[cfg(test)]
mod tests;
mod turn_end;

use crate::utils::{StreamNode, StreamNodeClock};
use assistant::{
    build_assistant_message, current_provider_model, partial_assistant_message,
    streaming_assistant_parts, streaming_message_update, upsert_streaming_tool,
};
use support::{
    ClearActiveTurn, attempt_model_stream, observe_script_hook, persist_agent_message,
    persist_agent_message_with_thought_elapsed, prepare_turn_binding,
};
use turn_end::{
    FinishTurnOutcome, drain_queue, finish_turn, queue_counts, try_turn_end_compaction,
};

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::Stream;
use futures::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::retry::{RetryState, is_retryable_error};
use super::state::{
    FrozenRootConfig, RunId, RunLease, RunPolicy, RuntimeControlError, SharedRunCoordinator,
};
use super::{AgentHooks, XyEvent, XyEventStream};
use crate::agent::capabilities::{AgentCapabilities, PendingMessageQueue};
use crate::agent::compaction::CompactionError;
use crate::agent::prompt::expand_skills_in_agent_messages;
use crate::agent::tools::ToolSet;
use crate::protocol::error::{XyError, XySessionError};
use crate::protocol::message::{AgentMessage, AgentPart};
use crate::protocol::model::{XyChunk, XyToolSchema};
use crate::protocol::ports::{
    XyBatchMode, XyHookBus, XyHookOutcome, XyModel, XySessionStore, XyStream,
};
use crate::protocol::resource::SkillInfo;

// ── AgentRuntime ───────────────────────────────────────────────────────

/// Session-bound ReAct actor: one bound session, one live root turn at a time.
pub struct AgentRuntime {
    inner: AgentCapabilities,
    coordinator: SharedRunCoordinator,
}

impl AgentRuntime {
    pub(crate) fn new(mut inner: AgentCapabilities) -> Self {
        let coordinator = SharedRunCoordinator::new();
        let probe_coord = coordinator.clone();
        inner.set_midturn_active_probe(Arc::new(move || {
            probe_coord.with(|c| c.has_active_run() || c.has_active_turn())
        }));
        Self { inner, coordinator }
    }

    /// Explicitly bind this actor to a session. Only allowed while idle.
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

    /// Toggle process obs-slot writes for `bind_session` (otel25). Host reader
    /// drivers disable this so read-only RPCs cannot stomp another session's id.
    pub(crate) fn set_obs_slot_writes(&mut self, enabled: bool) {
        self.inner.set_obs_slot_writes(enabled);
    }

    /// Whether `bind_session` on this runtime writes the process obs slot.
    pub(crate) fn obs_slot_writes(&self) -> bool {
        self.inner.obs_slot_writes()
    }

    /// Currently bound session id, if any.
    pub fn session_id(&self) -> Option<&str> {
        self.inner.session_id()
    }

    /// Cancellation token for the active (or last) root turn.
    pub fn cancel_token(&self) -> CancellationToken {
        self.coordinator.with(|c| c.cancel_token())
    }

    /// Abort the active root turn. Clears steer; keeps follow-up and queued roots.
    pub fn abort(&self) {
        self.coordinator.with_mut(|c| {
            c.cancel_active(false);
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

    pub fn session_store(&self) -> Arc<dyn crate::protocol::ports::XySessionStore> {
        self.inner.session_store()
    }

    pub fn set_tools(&mut self, tools: ToolSet) {
        self.inner.set_tools(tools);
    }

    pub fn set_tools_defer_prompt(
        &mut self,
        tools: ToolSet,
    ) -> crate::agent::prompt::SystemPromptOpts {
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

    pub fn freeze_tools_defer_prompt(
        &mut self,
        tools: ToolSet,
    ) -> crate::agent::prompt::SystemPromptOpts {
        self.inner.freeze_tools_defer_prompt(tools)
    }

    pub fn frozen_fingerprint_matches_set(&self, candidate: &ToolSet) -> bool {
        self.inner.frozen_fingerprint_matches_set(candidate)
    }

    pub fn replace_hooks(&mut self, hooks: AgentHooks) {
        self.inner.replace_hooks(hooks);
    }

    pub fn add_hook(&mut self, hook: super::hooks::BeforeToolHook) {
        self.inner.hooks_mut().add_before(hook);
    }

    pub fn set_should_stop_after_turn(
        &mut self,
        hook: Option<super::hooks::ShouldStopAfterTurnHook>,
    ) {
        self.inner.hooks_mut().set_should_stop_after_turn(hook);
    }

    pub fn set_permission(&mut self, permission: Arc<dyn crate::protocol::ports::XyPermission>) {
        self.inner.set_permission(permission);
    }

    pub fn set_tool_mode(&mut self, mode: crate::protocol::ports::XyBatchMode) {
        self.inner.set_tool_mode(mode);
    }

    pub fn set_batch_mode(&mut self, mode: crate::protocol::ports::XyBatchMode) {
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

    pub fn apply_skills(&mut self, skills: Vec<crate::protocol::resource::SkillInfo>) {
        self.inner.apply_skills(skills);
    }

    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.inner.loaded_skill_names()
    }

    pub fn loaded_skills(&self) -> Vec<crate::protocol::resource::SkillInfo> {
        self.inner.loaded_skills().to_vec()
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.inner.system_prompt()
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

    pub fn hook_bus(&self) -> Option<Arc<dyn XyHookBus>> {
        self.inner.hook_bus()
    }

    /// Cancel-path script hook (pre-flight): dispatch on the hook bus when
    /// present; a `Blocked` outcome maps to [`HookBlockedError`]. No bus is a
    /// no-op `Ok`.
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

    /// Observe-path script hook: fail-open (`Blocked` only logs). No bus is a
    /// no-op.
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

    pub fn current_model(&self) -> Option<crate::protocol::model::XyModelMeta> {
        self.inner.current_model()
    }

    pub fn model_registry(&self) -> crate::agent::model::registry::ModelRegistry {
        self.inner.model_registry()
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

    pub async fn cycle_thinking_level(&mut self) -> Result<String, XyError> {
        self.inner.cycle_thinking_level().await
    }

    pub fn apply_default_thinking_level(&mut self, raw: Option<&str>) {
        self.inner.apply_default_thinking_level(raw);
    }

    pub fn set_thinking_budgets(
        &mut self,
        budgets: Option<crate::protocol::model::ThinkingBudgets>,
    ) {
        self.inner.set_thinking_budgets(budgets);
    }

    pub(crate) fn restore_thinking_level(&mut self, level: String) {
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
        position: crate::protocol::session::ForkPosition,
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

    /// Submit a text root turn for the bound session.
    pub async fn submit_root(&mut self, prompt: &str, policy: RunPolicy) -> XyEventStream {
        self.submit_parts(
            vec![crate::protocol::message::AgentPart::text(prompt)],
            policy,
        )
        .await
    }

    /// Submit a multi-part root turn for the bound session.
    pub async fn submit_parts(
        &mut self,
        parts: Vec<crate::protocol::message::AgentPart>,
        policy: RunPolicy,
    ) -> XyEventStream {
        let Some(session_id) = self.inner.session_id().map(str::to_string) else {
            return XyEventStream::error(crate::protocol::lifecycle::XyEventError::new(
                "NoSession",
                "no session bound; call bind_session first",
            ));
        };

        let frozen = self.freeze_root_config(parts);

        // Policy decision before any await / mutation beyond the frozen snapshot.
        enum Admit {
            Immediate {
                run_id: RunId,
                cancel: CancellationToken,
            },
            Pending {
                run_id: RunId,
                rx: tokio::sync::oneshot::Receiver<FrozenRootConfig>,
            },
            Busy,
        }

        let admit = self.coordinator.with_mut(|c| {
            if c.phase().is_idle() {
                let (run_id, cancel) = c.try_begin_immediate().expect("idle begin");
                return Admit::Immediate { run_id, cancel };
            }
            match policy {
                RunPolicy::Reject => Admit::Busy,
                RunPolicy::AbortAndReplace => {
                    c.cancel_active(true);
                    let (run_id, rx) = c.enqueue_pending(frozen.clone(), false);
                    Admit::Pending { run_id, rx }
                }
                RunPolicy::QueueAfterRun => {
                    let (run_id, rx) = c.enqueue_pending(frozen.clone(), false);
                    Admit::Pending { run_id, rx }
                }
            }
        });

        match admit {
            Admit::Busy => XyEventStream::busy(),
            Admit::Immediate { run_id, cancel } => {
                self.start_root_stream(session_id, run_id, cancel, frozen)
                    .await
            }
            Admit::Pending { run_id, rx } => self.pending_root_stream(session_id, run_id, rx).await,
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
            permission: self.inner.get_permission(),
            hook_bus: self.inner.hook_bus(),
            cwd: self.inner.cwd().to_string(),
        }
    }

    async fn pending_root_stream(
        &self,
        session_id: String,
        run_id: RunId,
        rx: tokio::sync::oneshot::Receiver<FrozenRootConfig>,
    ) -> XyEventStream {
        let coordinator = self.coordinator.clone();
        let store = self.inner.session_store();
        let model_manager = self.inner.model_manager_handle();
        let queues = self.inner.queues();
        let event_sink_inner = self.inner.event_sink();
        let cwd = self.inner.cwd().to_string();

        let lease = RunLease::new(coordinator.clone(), run_id, Some(queues.clone()));

        let inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>> = Box::pin(async_stream::stream! {
            let Ok(frozen) = rx.await else {
                // Revoked (drop) or replace cleared pending — end quietly.
                return;
            };
            let cancel = match coordinator.with(|c| c.take_started_token(run_id)) {
                Some(t) => t,
                None => return,
            };

            if !store.exists(&session_id).await
                && let Err(e) = store.create(&session_id, Some(&cwd), None).await
            {
                yield XyEvent::Error(crate::protocol::lifecycle::XyEventError::from_xy(
                    &XyError::from(e),
                ));
                return;
            }

            let seeded_history = match load_history(&store, &session_id).await {
                Ok(h) => h,
                Err(e) => {
                    yield XyEvent::Error(crate::protocol::lifecycle::XyEventError::from_xy(&e));
                    return;
                }
            };

            coordinator.with_mut(|c| c.mark_streaming(run_id));
            let stream = build_live_react_stream(LiveReactArgs {
                run_id,
                coordinator: coordinator.clone(),
                cancel,
                frozen,
                model_manager,
                queues: queues.clone(),
                store,
                session_id,
                seeded_history,
                event_sink_inner,
            });
            let mut stream = std::pin::pin!(stream);
            while let Some(ev) = stream.next().await {
                yield ev;
            }
        });

        XyEventStream::with_lease(inner, lease)
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
            return XyEventStream::error(crate::protocol::lifecycle::XyEventError::from_xy(&e));
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
                return XyEventStream::error(crate::protocol::lifecycle::XyEventError::from_xy(&e));
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
        });
        self.coordinator.with_mut(|c| c.mark_streaming(run_id));
        XyEventStream::with_lease(Box::pin(stream), lease)
    }
}

async fn load_history(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
) -> Result<Vec<AgentMessage>, XyError> {
    let entries = store
        .load_leaf_branch(session_id)
        .await
        .map_err(XyError::from)?;
    let entries = crate::protocol::session::build_context_entries(&entries);
    Ok(entries
        .iter()
        .filter_map(|e| e.as_agent_message())
        .collect())
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
    event_sink_inner: Arc<dyn crate::protocol::ports::XyEventSink>,
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
    } = args;

    let FrozenRootConfig {
        user_parts,
        system_prompt,
        tools,
        hooks,
        batch_mode,
        skills,
        compaction_settings,
        permission,
        hook_bus,
        cwd,
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
        .map(|t| XyToolSchema {
            name: t.name().to_string(),
            description: t.description().to_string(),
            parameters: t.parameters_schema(),
        })
        .collect();
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
    let (side_tx, mut side_rx) = tokio::sync::mpsc::unbounded_channel::<XyEvent>();
    let event_sink: Arc<dyn crate::protocol::ports::XyEventSink> = Arc::new(CompactionStreamTee {
        inner: event_sink_inner,
        tx: side_tx,
    });

    let (queue_tx, mut queue_rx) = tokio::sync::mpsc::unbounded_channel();
    queues.bind_event_tx(run_id, queue_tx);

    let react = Box::pin(run_react_loop(ReActConfig {
        run_id,
        coordinator,
        model_manager,
        system_prompt,
        tools,
        tool_schemas,
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
        cwd,
    }));

    async_stream::stream! {
        let mut react = react;
        loop {
            tokio::select! {
                biased;
                ev = react.next() => {
                    match ev {
                        Some(e) => yield e,
                        None => break,
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
        // Queue unbind is owned by RunLease (AgentEnd / Drop), not this tail —
        // XyEventStream stops polling after AgentEnd so this block may not run.
    }
}

/// Forward CompactionStart/End from the side lifecycle sink onto the turn stream
/// so product TUI bridge can render the compaction block mid-run (c1730).
struct CompactionStreamTee {
    inner: Arc<dyn crate::protocol::ports::XyEventSink>,
    tx: tokio::sync::mpsc::UnboundedSender<XyEvent>,
}

#[async_trait::async_trait]
impl crate::protocol::ports::XyEventSink for CompactionStreamTee {
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

// ── Core ReAct loop config ─────────────────────────────────────────

/// Parameters for the ReAct agent loop.
struct ReActConfig {
    run_id: RunId,
    coordinator: SharedRunCoordinator,
    /// Shared selected model/thinking; refreshed at each turn boundary (c1470).
    model_manager: Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    /// System prompt snapshot for this run (ar6: next-run only).
    system_prompt: Option<String>,
    tools: ToolSet,
    tool_schemas: Vec<XyToolSchema>,
    user_parts: Vec<crate::protocol::message::AgentPart>,
    cancel: CancellationToken,
    /// Optional permission check. Called with (tool_name, target_path_or_domain).
    /// Returns Some(reason) if the operation is denied.
    #[allow(clippy::type_complexity)]
    permission_check: Option<std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>>,
    /// Hooks consulted at tool-call boundaries and optional after-turn stop.
    hooks: AgentHooks,
    /// Optional script hook bus (pi-aligned lifecycle + tool/context bridge).
    hook_bus: Option<Arc<dyn XyHookBus>>,
    /// Tool batch scheduling mode snapshot for this run (c1545).
    batch_mode: XyBatchMode,
    steer_queue: Arc<Mutex<PendingMessageQueue>>,
    follow_up_queue: Arc<Mutex<PendingMessageQueue>>,
    store: Arc<dyn XySessionStore>,
    session_id: String,
    seeded_history: Vec<AgentMessage>,
    /// Trust-filtered catalog for `$skill` expand (c1130); clone kept raw in history.
    skills: Vec<SkillInfo>,
    /// Compaction lifecycle sink (Start/End).
    event_sink: Arc<dyn crate::protocol::ports::XyEventSink>,
    /// Snapshot of compaction settings for turn-end threshold auto (c1640).
    compaction_settings: crate::agent::compaction::CompactionSettings,
    /// Workspace cwd for session_env (c1905).
    cwd: String,
}

// ── Core ReAct loop ─────────────────────────────────────────────────

fn run_react_loop(cfg: ReActConfig) -> impl Stream<Item = XyEvent> + Send {
    let ReActConfig {
        run_id,
        coordinator,
        model_manager,
        system_prompt,
        tools,
        tool_schemas,
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
        cwd,
    } = cfg;
    async_stream::stream! {
        let _clear_active = ClearActiveTurn {
            coordinator: coordinator.clone(),
            run_id,
        };

        if let Some(bus) = &hook_bus {
            let (ty, phase, ctx) = super::script_hook_ctx::agent_start();
            observe_script_hook(bus, ty, phase, ctx).await;
        }

        let mut history: Vec<AgentMessage> = seeded_history;
        // System prompt rides on `generate_options.system_prompt` (c1270 / pi align).
        // MUST NOT stuff it into history as a fake user turn.
        // pi `newMessages`: everything this run appends (exclude pre-seed).
        let run_baseline = history.len();

        // c1905/c1906: session_env bootstrap — ensure then persist when appended.
        let env_snap = crate::agent::prompt::snapshot_for_cwd(&cwd);
        if crate::agent::prompt::ensure_session_env_in_history(&mut history, &env_snap) {
            persist_agent_message(&store, &session_id, history.last().expect("session_env")).await;
        }

        // Add user message (text and/or images, c1155).
        history.push(AgentMessage::user_parts(user_parts.clone()));
        persist_agent_message(&store, &session_id, history.last().expect("user message")).await;

        let retry_state = RetryState::new(3, 1000);
        // Steering queued before/at run start is injected before the first model call.
        let mut pending: Vec<AgentMessage> = drain_queue(&steer_queue);
        let mut turn: usize = 0;
        // Per-run model instance: reuse while selected id is unchanged (NextTurn).
        let mut run_model: Option<(String, Arc<dyn XyModel>)> = None;
        // c1660: at most one overflow compact-and-retry per run.
        let mut overflow_recovery_attempted = false;
        let mut stream_clock = StreamNodeClock::new();
        stream_clock.stamp(StreamNode::AgentStart);
        // One OTEL/fastrace tree per user-triggered run (c1495 / c1555 turn preview).
        let user_preview = super::tool_exec::parts_preview_text(&user_parts);
        let model_api = {
            let mm = crate::utils::lock_mutex(&model_manager);
            mm.current_model().map(|m| m.api.clone())
        };
        let (parent_session_id, fork_at_entry_id) = store
            .load_entries(&session_id)
            .await
            .ok()
            .map(|entries| crate::protocol::session::session_fork_edge(&entries))
            .unwrap_or((None, None));
        let obs_session = xylitol_ai_bridge::ObsSessionContext {
            session_id: Some(session_id.clone()),
            session_name: xylitol_ai_bridge::provider::obs_session_context().session_name,
            parent_session_id,
            fork_at_entry_id,
            llm_gateway_session_id: None,
        };
        let agent_turn_span = super::obs::AgentTurnSpan::start_with_session(
            Some(user_preview.as_str()),
            model_api.as_deref(),
            &obs_session,
        );
        let turn_obs_parent = super::tool_exec::capture_iteration_parent(
            agent_turn_span.as_ref().map(|s| s.span()),
        );
        // c1720: mark turn root aborted when cancel token ends the run.
        let mut turn_aborted = false;

        // Outer loop: continues when follow-up messages arrive after the agent
        // would otherwise stop (pi runLoop semantics).
        'outer: loop {
            let mut continue_after_tools = true;

            while continue_after_tools || !pending.is_empty() {
                if cancel.is_cancelled() {
                    // Bridge maps this to a dim scroll notice + idle (not a sticky fault).
                    turn_aborted = true;
                    yield XyEvent::aborted();
                    break 'outer;
                }

                stream_clock.begin_turn();
                stream_clock.stamp(StreamNode::TurnStart);
                yield XyEvent::TurnStart { turn_index: turn as u32 };
                if let Some(bus) = &hook_bus {
                    let (ty, phase, ctx) = super::script_hook_ctx::turn_start(turn as u32);
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

                // Inject pending messages (steering / follow-up) before the model call.
                // Emit user MessageStart/End so surfaces can 上行 scrollback (pi chat).
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
                        yield XyEvent::MessageEnd {
                            role: "user".to_string(),
                            message: Some(message.clone()),
                        };
                        if let Some(bus) = &hook_bus {
                            let (ty, phase, ctx) = super::script_hook_ctx::message_end("user");
                            observe_script_hook(bus, ty, phase, ctx).await;
                        }
                        history.push(message.clone());
                        persist_agent_message(&store, &session_id, &message).await;
                    }
                    let (steer_count, follow_up_count) =
                        queue_counts(&steer_queue, &follow_up_queue);
                    yield XyEvent::QueueUpdate {
                        steer_count,
                        follow_up_count,
                    };
                }

                let mut messages = history.clone();
                // Expand `$skill` only on the model-bound clone (history stays raw).
                expand_skills_in_agent_messages(&mut messages, &skills);
                if let Some(bus) = &hook_bus
                    && let (ty, phase, ctx) =
                        super::script_hook_ctx::context_pre(messages.len())
                    && let XyHookOutcome::Blocked { reason } =
                        bus.dispatch(ty, phase, ctx).await
                {
                    yield XyEvent::error_msg(format!("context hook blocked: {reason}"));
                    break 'outer;
                }

                // NextTurn: re-read selected model + thinking at turn boundary (c1470).
                let (model, mut generate_options) = match prepare_turn_binding(
                    &model_manager,
                    &coordinator,
                    run_id,
                    &system_prompt,
                    &mut run_model,
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        yield XyEvent::error_msg(format!("model build error: {e}"));
                        break 'outer;
                    }
                };
                generate_options.obs_parent = iteration_parent;
                generate_options.obs_session = obs_session.clone();

                // Race cancel against connect/retry so Esc aborts hung `send()`
                // (reqwest drop-cancels the in-flight HTTP future).
                // 重试环在生成器内联:AutoRetryStart 在 backoff 等待「前」yield,
                // bridge 才能在等待期间显示 Retry attempt/max(atb6)。
                let llm_messages = crate::agent::llm_project::project_for_llm(&messages);
                let mut stream_result: Option<Result<XyStream, XyError>> = None;
                loop {
                    let result = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => break,
                        result = attempt_model_stream(
                            &model, llm_messages.clone(), &tool_schemas, &generate_options,
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
                            if is_retryable_error(&e.to_string()) && retry_state.can_retry() {
                                log::warn!(
                                    target: "xylitol::react",
                                    "model.generate_stream retrying error.kind={} turn_id={} error={e}",
                                    e.kind(),
                                    turn_id.as_deref().unwrap_or("")
                                );
                                let delay = retry_state.next_delay();
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
                        turn_aborted = true;
                        yield XyEvent::aborted();
                        break 'outer;
                    }
                    Some(Ok(s)) => {
                        chunk_stream = s;
                    }
                    Some(Err(e)) => {
                        // c1660: overflow on connect → synthesize error assistant + Case1.
                        let err_text = e.to_string();
                        let (provider, model_id) = current_provider_model(&model_manager);
                        let err_asst = build_assistant_message(
                            vec![AgentPart::text("")],
                            Some(crate::protocol::message::XyStopReason::Error),
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
                            &mut history,
                            &mut overflow_recovery_attempted,
                            None,
                            turn_obs_parent,
                            &obs_session,
                            &cwd,
                        )
                        .await;
                        if will_continue {
                            continue 'outer;
                        }
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
                let mut done_usage: Option<crate::protocol::message::XyUsage> = None;
                let mut done_stop_reason: Option<crate::protocol::message::XyStopReason> = None;
                let mut done_error_message: Option<String> = None;

                // Mid-stream abort: drop `chunk_stream` so adapter/reqwest closes
                // the HTTP body (c680). Surfaces inherit via XyDriver::abort → token.
                loop {
                    let chunk_result = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => {
                            drop(chunk_stream);
                            // c1595 / pi: keep partial assistant in session with
                            // stop_reason=Aborted (skip empty); project_for_llm
                            // filters it from the next model call.
                            let assistant_parts = streaming_assistant_parts(
                                &text_acc,
                                &thinking_acc,
                                thinking_signature.as_deref(),
                                &tool_calls,
                            );
                            if !assistant_parts.is_empty() {
                                let assistant_msg = build_assistant_message(
                                    assistant_parts,
                                    Some(crate::protocol::message::XyStopReason::Aborted),
                                    done_usage.take(),
                                    String::new(),
                                    String::new(),
                                    None,
                                );
                                yield XyEvent::MessageEnd {
                                    role: "assistant".to_string(),
                                    message: Some(assistant_msg.clone()),
                                };
                                stream_clock.finish_message();
                                persist_agent_message_with_thought_elapsed(
                                    &store,
                                    &session_id,
                                    &assistant_msg,
                                    Some(&stream_clock),
                                )
                                .await;
                                history.push(assistant_msg);
                            }
                            turn_aborted = true;
                            yield XyEvent::aborted();
                            break 'outer;
                        }
                        next = chunk_stream.next() => next,
                    };
                    match chunk_result {
                        None => break,
                        Some(Ok(chunk)) => match chunk {
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
                                // Intent only — execute after MessageEnd (c1255 / ar21).
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
                                // Persist usage/stop for Api-anchor estimates (c1420 / ar23).
                                done_stop_reason = Some(finish_reason);
                                if usage.is_some() {
                                    done_usage = usage;
                                }
                            }
                        },
                        Some(Err(e)) => {
                            super::obs::record_xy_error(
                                "model.stream",
                                &e,
                                turn_id.as_deref(),
                                iteration_parent,
                                &obs_session,
                            );
                            let err_text = format!("stream error: {e}");
                            done_stop_reason =
                                Some(crate::protocol::message::XyStopReason::Error);
                            done_error_message = Some(err_text.clone());
                            yield XyEvent::error_msg(err_text);
                            break;
                        }
                    }
                }

                let assistant_partial = if text_acc.is_empty()
                    && thinking_acc.is_empty()
                    && thinking_signature.is_none()
                    && tool_calls.is_empty()
                {
                    None
                } else {
                    Some(partial_assistant_message(
                        &text_acc,
                        &thinking_acc,
                        thinking_signature.as_deref(),
                        &tool_calls,
                    ))
                };
                yield XyEvent::MessageEnd {
                    role: "assistant".to_string(),
                    message: assistant_partial,
                };
                if let Some(bus) = &hook_bus {
                    let (ty, phase, ctx) = super::script_hook_ctx::message_end("assistant");
                    observe_script_hook(bus, ty, phase, ctx).await;
                }

                let mut assistant_parts = streaming_assistant_parts(
                    &text_acc,
                    &thinking_acc,
                    thinking_signature.as_deref(),
                    &tool_calls,
                );
                if !assistant_parts.is_empty()
                    || matches!(
                        done_stop_reason,
                        Some(crate::protocol::message::XyStopReason::Error)
                    )
                {
                    let (provider, model_id) = current_provider_model(&model_manager);
                    if assistant_parts.is_empty() {
                        assistant_parts.push(AgentPart::text(""));
                    }
                    let assistant_msg = build_assistant_message(
                        assistant_parts,
                        done_stop_reason,
                        done_usage,
                        provider,
                        model_id,
                        done_error_message,
                    );
                    stream_clock.finish_message();
                    persist_agent_message_with_thought_elapsed(
                        &store,
                        &session_id,
                        &assistant_msg,
                        Some(&stream_clock),
                    )
                    .await;
                    history.push(assistant_msg);
                }

                continue_after_tools = !tool_calls.is_empty();

                if tool_calls.is_empty() {
                    let assistant = history
                        .iter()
                        .rev()
                        .find(|m| m.role_name() == "assistant")
                        .cloned();
                    let finished = finish_turn(
                        &store,
                        &session_id,
                        &model_manager,
                        &event_sink,
                        &compaction_settings,
                        &mut history,
                        &mut overflow_recovery_attempted,
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
                    )
                    .await;
                    for event in finished.events {
                        yield event;
                    }
                    match finished.outcome {
                        FinishTurnOutcome::ContinueOuterForCompaction => continue 'outer,
                        FinishTurnOutcome::StopRun => break 'outer,
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
                            continue;
                        }
                    }
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
                    obs_session: &obs_session,
                };
                let parent_ctx = iteration_parent;

                // Collect (window_index, call indices) for Sequential as one Barrier each,
                // or BarrierParallel via plan_windows.
                let planned: Vec<super::tool_batch::PlannedWindow> = match batch_mode {
                    XyBatchMode::Sequential => tool_calls
                        .iter()
                        .enumerate()
                        .map(|(i, _)| super::tool_batch::PlannedWindow::Barrier(i))
                        .collect(),
                    XyBatchMode::BarrierParallel => {
                        let classes: Vec<_> = tool_calls
                            .iter()
                            .map(|(_, name, _)| {
                                let tool = tools.get(name);
                                super::tool_batch::classify(
                                    name,
                                    tool.as_ref().map(|t| t.as_ref() as &dyn crate::protocol::ports::XyTool),
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
                            let (id, name, args) = &tool_calls[i];
                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                            let fut = super::tool_exec::run_one(
                                &tool_env,
                                id,
                                name,
                                args,
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
                            history.push(msg.clone());
                            persist_agent_message(
                                &store,
                                &session_id,
                                history.last().expect("tool result"),
                            )
                            .await;
                            turn_tool_results.push(msg);
                        }
                        super::tool_batch::PlannedWindow::Parallel(idxs) => {
                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                            let mut futs = Vec::with_capacity(idxs.len());
                            for &i in &idxs {
                                let (id, name, args) = &tool_calls[i];
                                let tx = tx.clone();
                                futs.push(async {
                                    super::tool_exec::run_one(
                                        &tool_env,
                                        id,
                                        name,
                                        args,
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
                            // History / toolResults: source order (join_all preserves idxs order).
                            for msg in msgs {
                                history.push(msg.clone());
                                persist_agent_message(
                                    &store,
                                    &session_id,
                                    history.last().expect("tool result"),
                                )
                                .await;
                                turn_tool_results.push(msg);
                            }
                        }
                    }
                }

                let finished = finish_turn(
                    &store,
                    &session_id,
                    &model_manager,
                    &event_sink,
                    &compaction_settings,
                    &mut history,
                    &mut overflow_recovery_attempted,
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
                )
                .await;
                for event in finished.events {
                    yield event;
                }
                match finished.outcome {
                    FinishTurnOutcome::ContinueOuterForCompaction => continue 'outer,
                    FinishTurnOutcome::StopRun => break 'outer,
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

            // Would stop — drain follow-up; if non-empty, continue outer loop.
            let follow_ups = drain_queue(&follow_up_queue);
            if follow_ups.is_empty() {
                break;
            }
            pending = follow_ups;
            let (steer_count, follow_up_count) = queue_counts(&steer_queue, &follow_up_queue);
            yield XyEvent::QueueUpdate {
                steer_count,
                follow_up_count,
            };
        }

        if let Some(bus) = &hook_bus {
            // pi agent_settled: no retry/compaction/follow-up left before AgentEnd.
            let (ty, phase, ctx) = super::script_hook_ctx::agent_settled();
            observe_script_hook(bus, ty, phase, ctx).await;
            let (ty, phase, ctx) = super::script_hook_ctx::agent_end();
            observe_script_hook(bus, ty, phase, ctx).await;
        }
        // Keep `agent.turn` open for the whole run (NLL would otherwise drop early).
        // c1720 / otel20: explicit terminal status before span drop.
        if let Some(span) = agent_turn_span {
            use super::obs::TurnEndReason;
            span.finish(if turn_aborted {
                TurnEndReason::Aborted
            } else {
                TurnEndReason::Ok
            });
        }
        yield XyEvent::AgentEnd { messages: history };
    }
}
