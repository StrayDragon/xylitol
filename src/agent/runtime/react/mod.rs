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
mod support;
#[cfg(test)]
mod tests;
mod turn_end;

use assistant::{
    build_assistant_message, current_provider_model, partial_assistant_message,
    streaming_assistant_parts, streaming_message_update, upsert_streaming_tool,
};
use support::{
    ClearActiveTurn, call_with_retry, observe_script_hook, persist_agent_message,
    prepare_turn_binding,
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

use super::retry::RetryState;
use super::{AgentHooks, XyEvent, XyEventStream};
use crate::agent::capabilities::{AgentCapabilities, PendingMessageQueue};
use crate::agent::prompt::expand_skills_in_agent_messages;
use crate::agent::tools::ToolSet;
use crate::protocol::error::XyError;
use crate::protocol::message::{AgentMessage, AgentPart};
use crate::protocol::model::{XyChunk, XyToolSchema};
use crate::protocol::ports::{XyBatchMode, XyHookBus, XyHookOutcome, XyModel, XySessionStore};
use crate::protocol::resource::SkillInfo;

// ── AgentRuntime ───────────────────────────────────────────────────────

pub struct AgentRuntime {
    pub(crate) inner: AgentCapabilities,
    /// Current-run cancel token. Replaced at each [`Self::run`] so abort is not sticky.
    cancel: Mutex<CancellationToken>,
}

impl AgentRuntime {
    pub fn new(inner: AgentCapabilities) -> Self {
        Self {
            inner,
            cancel: Mutex::new(CancellationToken::new()),
        }
    }

    /// Get a reference to the cancellation token for the active (or last) run.
    pub fn cancel_token(&self) -> CancellationToken {
        crate::utils::lock_mutex(&self.cancel).clone()
    }

    /// Signal cancellation to abort the agent loop.
    ///
    /// Clears the steering queue and keeps follow-up messages so the UI can
    /// restore them (c461 design D4). Only cancels the **current** run token;
    /// the next [`Self::run`] installs a fresh one (c482). Also cancels any
    /// Mid-stream model HTTP is aborted by racing this token in the ReAct chunk
    /// loop and dropping the provider stream (c680; surfaces inherit via
    /// [`crate::app::core::driver::XyDriver::abort`]). Interactive bang cancel
    /// is owned by [`XyInProcessDriver::abort`](crate::app::core::driver::XyInProcessDriver)
    /// (app-surface), not the ReAct runtime.
    pub fn abort(&self) {
        crate::utils::lock_mutex(&self.cancel).cancel();
        self.inner.clear_steer_queue();
        self.inner.clear_active_turn();
    }

    /// Enqueue a steering message for the active (or next) run.
    pub fn steer(&self, message: impl Into<String>) {
        self.inner.steer(message);
    }

    /// Enqueue a follow-up message delivered when the run would otherwise stop.
    pub fn follow_up(&self, message: impl Into<String>) {
        self.inner.follow_up(message);
    }

    /// Clear one or both pending-message queues.
    pub fn clear_queues(&self, clear_steer: bool, clear_follow_up: bool) {
        self.inner.clear_queues(clear_steer, clear_follow_up);
    }

    /// Queue depths.
    pub fn queue_stats(&self) -> crate::agent::capabilities::QueueStats {
        self.inner.queue_stats()
    }

    pub fn inner(&self) -> &AgentCapabilities {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut AgentCapabilities {
        &mut self.inner
    }

    /// Session store shared with the XyDriver seam.
    pub fn session_store(&self) -> Arc<dyn crate::protocol::ports::XySessionStore> {
        self.inner.session_store()
    }

    /// Replace the tool set. Takes effect on the next [`run`](Self::run) call.
    /// Ignored while the tool table is FROZEN (c1900); use [`Self::freeze_tools`].
    pub fn set_tools(&mut self, tools: ToolSet) {
        self.inner.set_tools(tools);
    }

    /// Install tools without rebuilding system prompt text (MCP settle offload).
    /// Ignored while FROZEN (c1900).
    pub fn set_tools_defer_prompt(
        &mut self,
        tools: ToolSet,
    ) -> crate::agent::prompt::SystemPromptOpts {
        self.inner.set_tools_defer_prompt(tools)
    }

    /// Install a prebuilt system prompt (pair with [`Self::set_tools_defer_prompt`]).
    pub fn install_system_prompt_text(&mut self, prompt: String) {
        self.inner.install_system_prompt_text(prompt);
    }

    /// Track-A freeze phase (c1900).
    pub fn tool_freeze_phase(&self) -> crate::agent::tools::ToolFreezePhase {
        self.inner.tool_freeze_phase()
    }

    /// True when provider-visible tools are frozen.
    pub fn is_tools_frozen(&self) -> bool {
        self.inner.is_tools_frozen()
    }

    pub fn frozen_tool_fingerprint(&self) -> Option<&crate::agent::tools::ToolTableFingerprint> {
        self.inner.frozen_tool_fingerprint()
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

    /// Freeze provider-visible tools (bypasses FROZEN ignore on [`Self::set_tools`]).
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

    /// Replace the hook set. Takes effect on the next [`run`](Self::run) call.
    pub fn replace_hooks(&mut self, hooks: AgentHooks) {
        self.inner.replace_hooks(hooks);
    }

    /// Add a before-tool hook. Takes effect on the next [`run`](Self::run) call.
    pub fn add_hook(&mut self, hook: super::hooks::BeforeToolHook) {
        self.inner.hooks_mut().add_before(hook);
    }

    /// Set the optional after-turn stop callback (pi `shouldStopAfterTurn`).
    ///
    /// Takes effect on the next [`run`](Self::run) call. Single slot — not a chain.
    pub fn set_should_stop_after_turn(
        &mut self,
        hook: Option<super::hooks::ShouldStopAfterTurnHook>,
    ) {
        self.inner.hooks_mut().set_should_stop_after_turn(hook);
    }

    /// Set the permission port. Takes effect on the next [`run`](Self::run) call.
    pub fn set_permission(&mut self, permission: Arc<dyn crate::protocol::ports::XyPermission>) {
        self.inner.set_permission(permission);
    }

    /// Set the tool batch mode. Takes effect on the next [`run`](Self::run) call.
    pub fn set_tool_mode(&mut self, mode: crate::protocol::ports::XyBatchMode) {
        self.inner.set_tool_mode(mode);
    }

    /// Set the tool batch mode (alias of [`Self::set_tool_mode`]).
    pub fn set_batch_mode(&mut self, mode: crate::protocol::ports::XyBatchMode) {
        self.inner.set_tool_mode(mode);
    }

    /// Set the system prompt. Takes effect on the next [`run`](Self::run) call.
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.inner.set_system_prompt(prompt);
    }

    /// Replace context / SYSTEM / APPEND and rebuild system prompt (c1100).
    /// Takes effect on the next [`run`](Self::run); does not mutate history.
    pub fn apply_prompt_resources(
        &mut self,
        context_files: Vec<(String, String)>,
        system_prompt: Option<String>,
        append_system_prompt: Vec<String>,
    ) {
        self.inner
            .apply_prompt_resources(context_files, system_prompt, append_system_prompt);
    }

    /// Replace skills catalog and rebuild system prompt (c1085).
    pub fn apply_skills(&mut self, skills: Vec<crate::protocol::resource::SkillInfo>) {
        self.inner.apply_skills(skills);
    }

    /// Names currently injected into the system prompt (c1085).
    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.inner.loaded_skill_names()
    }

    /// Full skill catalog for `$` completion / expand (c1130).
    pub fn loaded_skills(&self) -> &[crate::protocol::resource::SkillInfo] {
        self.inner.loaded_skills()
    }

    /// Run a turn with an auto-generated session_id.
    pub async fn run(&mut self, prompt: &str) -> XyEventStream {
        self.run_parts_with_id(
            vec![crate::protocol::message::AgentPart::text(prompt)],
            &uuid::Uuid::new_v4().to_string(),
        )
        .await
    }

    /// Run a multi-part user turn (text + images, c1155).
    pub async fn run_parts(
        &mut self,
        parts: Vec<crate::protocol::message::AgentPart>,
    ) -> XyEventStream {
        self.run_parts_with_id(parts, &uuid::Uuid::new_v4().to_string())
            .await
    }

    /// Run a turn with an explicit session_id.
    #[allow(clippy::type_complexity)]
    pub async fn run_with_id(&mut self, prompt: &str, session_id: &str) -> XyEventStream {
        self.run_parts_with_id(
            vec![crate::protocol::message::AgentPart::text(prompt)],
            session_id,
        )
        .await
    }

    /// Run a multi-part user turn with an explicit session_id (c1155).
    #[allow(clippy::type_complexity)]
    pub async fn run_parts_with_id(
        &mut self,
        parts: Vec<crate::protocol::message::AgentPart>,
        session_id: &str,
    ) -> XyEventStream {
        // Build permission check callback from session (capability map in permission_router).
        let permission_check: Option<
            std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>,
        >;
        {
            let engine = self.inner.get_permission();
            permission_check = Some(std::sync::Arc::new(
                move |tool_name: &str, tool_path: &str| -> Option<String> {
                    super::permission_router::check_tool_permission(
                        engine.as_ref(),
                        tool_name,
                        tool_path,
                    )
                },
            ));
        }
        // Ensure session exists
        let sid = session_id.to_string();
        self.inner.set_session(sid.clone());
        let t_ensure = std::time::Instant::now();
        if let Err(e) = self.inner.ensure_session(&sid, None).await {
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
        let seeded_history = match self.inner.load_conversation_history(&sid).await {
            Ok(h) => h,
            Err(e) => {
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

        let tools = self.inner.tools().clone();
        let hooks = self.inner.hooks().clone();
        let hook_bus = self.inner.hook_bus();
        let batch_mode = self.inner.tool_mode();
        let user_parts = parts;
        let model_manager = self.inner.model_manager_handle();
        let active_turn = self.inner.active_turn_handle();
        let system_prompt = self.inner.system_prompt().map(|s| s.to_string());

        // Build tool schemas
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

        let cancel = {
            let mut guard = crate::utils::lock_mutex(&self.cancel);
            *guard = CancellationToken::new();
            guard.clone()
        };
        let steer_queue = self.inner.steer_queue();
        let follow_up_queue = self.inner.follow_up_queue();
        let queues = self.inner.queues();
        let store = self.inner.session_store();
        let skills = self.inner.loaded_skills().to_vec();
        let (side_tx, mut side_rx) = tokio::sync::mpsc::unbounded_channel::<XyEvent>();
        let event_sink: Arc<dyn crate::protocol::ports::XyEventSink> =
            Arc::new(CompactionStreamTee {
                inner: self.inner.event_sink(),
                tx: side_tx,
            });
        let compaction_settings = self.inner.compaction_settings();

        let (queue_tx, mut queue_rx) = tokio::sync::mpsc::unbounded_channel();
        queues.bind_event_tx(queue_tx);

        let react = Box::pin(run_react_loop(ReActConfig {
            model_manager,
            active_turn,
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
            session_id: sid,
            seeded_history,
            skills,
            event_sink,
            compaction_settings,
        }));

        let inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>> = Box::pin(async_stream::stream! {
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
            queues.unbind_event_tx();
        });

        XyEventStream { inner, done: false }
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
    /// Shared selected model/thinking; refreshed at each turn boundary (c1470).
    model_manager: Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    /// Active in-flight binding for chrome; cleared when the run ends.
    active_turn: Arc<Mutex<Option<crate::agent::capabilities::ActiveTurnBinding>>>,
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
}

// ── Core ReAct loop ─────────────────────────────────────────────────

fn run_react_loop(cfg: ReActConfig) -> impl Stream<Item = XyEvent> + Send {
    let ReActConfig {
        model_manager,
        active_turn,
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
    } = cfg;
    async_stream::stream! {
        let _clear_active = ClearActiveTurn(active_turn.clone());

        if let Some(bus) = &hook_bus {
            let (ty, phase, ctx) = super::script_hook_ctx::agent_start();
            observe_script_hook(bus, ty, phase, ctx).await;
        }

        let mut history: Vec<AgentMessage> = seeded_history;
        // System prompt rides on `generate_options.system_prompt` (c1270 / pi align).
        // MUST NOT stuff it into history as a fake user turn.
        // pi `newMessages`: everything this run appends (exclude pre-seed).
        let run_baseline = history.len();

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
        // One OTEL/fastrace tree per user-triggered run (c1495 / c1555 turn preview).
        let user_preview = super::tool_exec::parts_preview_text(&user_parts);
        let model_api = {
            let mm = crate::utils::lock_mutex(&model_manager);
            mm.current_model().map(|m| m.api.clone())
        };
        let agent_turn_span =
            super::obs::AgentTurnSpan::start(Some(user_preview.as_str()), model_api.as_deref());
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

                yield XyEvent::TurnStart { turn_index: turn as u32 };
                if let Some(bus) = &hook_bus {
                    let (ty, phase, ctx) = super::script_hook_ctx::turn_start(turn as u32);
                    observe_script_hook(bus, ty, phase, ctx).await;
                }
                let iteration_span =
                    super::obs::AgentIterationSpan::start(agent_turn_span.as_ref(), turn);
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
                if !hooks.transform_context.is_empty() {
                    for hook in &hooks.transform_context {
                        messages = hook(messages);
                    }
                }
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
                let (model, generate_options) = match prepare_turn_binding(
                    &model_manager,
                    &active_turn,
                    &system_prompt,
                    &mut run_model,
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        yield XyEvent::error_msg(format!("model build error: {e}"));
                        break 'outer;
                    }
                };

                // Race cancel against connect/retry so Esc aborts hung `send()`
                // (reqwest drop-cancels the in-flight HTTP future).
                let stream_result = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => None,
                    result = call_with_retry(
                        &model, messages, &tool_schemas, &retry_state, &generate_options,
                        turn_id.as_deref(),
                    ) => Some(result),
                };

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
                                persist_agent_message(&store, &session_id, &assistant_msg)
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
                            super::obs::record_xy_error("model.stream", &e, turn_id.as_deref());
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
                    persist_agent_message(&store, &session_id, &assistant_msg).await;
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
                };
                let parent_ctx = super::tool_exec::capture_iteration_parent(
                    iteration_span.as_ref().map(|s| s.span()),
                );

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
        // c1720 / otel20: explicit terminal status before parent slots clear.
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
