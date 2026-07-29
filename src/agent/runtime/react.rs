//! Agent execution loop — core ReAct loop with full event stream, hooks, and tool batch modes.
//!
//! NOTE: 本文件聚焦 ReAct 算法主体. 天花板: ~500 行 (含 inline tests), 因 run_react_loop
//! 的 async_stream 宏块是原子逻辑单元, 跨函数 yield 不可行. 升级: 当工具执行/流处理逻辑
//! 显著膨胀时, 考虑引入 sub-turn state machine 替代单宏块.
//!
//! Tool batch scheduling lives in `tool_batch` + `tool_exec` (c1545). Product default is
//! Sequential (source-order await); BarrierParallel fans out ParallelSafe windows.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::Stream;
use futures::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

fn upsert_streaming_tool(
    tools: &mut Vec<(String, String, Value)>,
    id: String,
    name: String,
    args: Value,
) {
    if let Some(slot) = tools
        .iter_mut()
        .find(|(existing_id, _, _)| existing_id == &id)
    {
        slot.1 = name;
        slot.2 = args;
    } else {
        tools.push((id, name, args));
    }
}

fn partial_assistant_message(
    text: &str,
    thinking: &str,
    thinking_signature: Option<&str>,
    tool_calls: &[(String, String, Value)],
) -> AgentMessage {
    let mut parts = Vec::new();
    if !thinking.is_empty() || thinking_signature.is_some() {
        parts.push(AgentPart::Thinking {
            thinking: thinking.to_string(),
            redacted: false,
            thinking_signature: thinking_signature.map(str::to_string),
        });
    }
    if !text.is_empty() {
        parts.push(AgentPart::text(text.to_string()));
    }
    for (id, name, args) in tool_calls {
        parts.push(AgentPart::ToolCall {
            id: id.clone(),
            name: name.clone(),
            arguments: args.clone(),
        });
    }
    AgentMessage::Llm(LlmMessage::AssistantMessage {
        content: parts,
        stop_reason: None,
        usage: None,
        api: String::new(),
        provider: String::new(),
        model: String::new(),
        response_id: None,
        error_message: None,
        timestamp: crate::protocol::message::now_ms(),
        diagnostics: Vec::new(),
    })
}

use super::hooks::ShouldStopAfterTurnCtx;
use super::retry::{RetryState, is_retryable_error};
use super::{AgentHooks, XyEvent, XyEventStream};
use crate::agent::llm_project::project_for_llm;
use crate::agent::prompt::expand_skills_in_agent_messages;
use crate::agent::session::{AgentCapabilities, PendingMessageQueue};
use crate::agent::tools::ToolSet;
use crate::protocol::error::XyError;
use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};
use crate::protocol::ports::{XyBatchMode, XyHookBus, XyHookOutcome, XyModel, XySessionStore};
use crate::protocol::resource::SkillInfo;
use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};
use crate::protocol::types::{XyChunk, XyToolSchema};

use crate::protocol::ports::XyPermissionVerdict;

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
        self.cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Signal cancellation to abort the agent loop.
    ///
    /// Clears the steering queue and keeps follow-up messages so the UI can
    /// restore them (c461 design D4). Only cancels the **current** run token;
    /// the next [`Self::run`] installs a fresh one (c482). Also cancels any
    /// in-flight interactive `!`/`!!` bash (c660; aligns with pi `abortBash`).
    /// Mid-stream model HTTP is aborted by racing this token in the ReAct chunk
    /// loop and dropping the provider stream (c680; surfaces inherit via
    /// [`crate::app::core::driver::XyDriver::abort`]).
    pub fn abort(&self) {
        self.cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cancel();
        self.inner.clear_steer_queue();
        self.inner.abort_bash();
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
    pub fn queue_stats(&self) -> crate::agent::session::QueueStats {
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
    pub fn set_tools(&mut self, tools: ToolSet) {
        self.inner.set_tools(tools);
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
        // Build permission check callback from session
        let permission_check: Option<
            std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>,
        >;
        {
            let engine = self.inner.get_permission();
            permission_check = Some(std::sync::Arc::new(
                move |tool_name: &str, tool_path: &str| -> Option<String> {
                    // NOTE: tool_name → permission check dispatch is hard-coded here.
                    // Ceiling: adding a new sandbox-sensitive tool requires editing this match.
                    // Upgrade: when xylitol matures as a harness, introduce tool-declared
                    // capability categories and replace this match with a capability-driven router.
                    match tool_name {
                        "read" => match engine.check_read(tool_path) {
                            XyPermissionVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        "write" | "edit" => match engine.check_write(tool_path) {
                            XyPermissionVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        "bash" => match engine.check_network(tool_path) {
                            XyPermissionVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        _ => None,
                    }
                },
            ));
        }
        // Ensure session exists
        let sid = session_id.to_string();
        self.inner.set_session(sid.clone());
        if let Err(e) = self.inner.ensure_session(&sid, None).await {
            return XyEventStream::error(format!("session error: {e}"));
        }

        let seeded_history = match self.inner.load_conversation_history(&sid).await {
            Ok(h) => h,
            Err(e) => return XyEventStream::error(format!("session load error: {e}")),
        };

        let tools = self.inner.tools().clone();
        let hooks = self.inner.hooks().clone();
        let hook_bus = self.inner.hook_bus();
        let batch_mode = self.inner.tool_mode();
        let user_parts = parts;
        let model_manager = self.inner.model_manager_handle();
        let active_turn = self.inner.active_turn_handle();
        let system_prompt = self.inner.system_prompt().map(|s| s.to_string());

        // Build tool schemas
        let tool_schemas: Vec<XyToolSchema> = tools
            .iter()
            .map(|t| XyToolSchema {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();

        let cancel = {
            let mut guard = self.cancel.lock().unwrap_or_else(|e| e.into_inner());
            *guard = CancellationToken::new();
            guard.clone()
        };
        let steer_queue = self.inner.steer_queue();
        let follow_up_queue = self.inner.follow_up_queue();
        let queues = self.inner.queues();
        let store = self.inner.session_store();
        let skills = self.inner.loaded_skills().to_vec();
        let event_sink = self.inner.event_sink();
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
                }
            }
            queues.unbind_event_tx();
        });

        XyEventStream {
            inner,
            done: false,
            turn_index: 0,
        }
    }
}

// ── Core ReAct loop config ─────────────────────────────────────────

/// Parameters for the ReAct agent loop.
struct ReActConfig {
    /// Shared selected model/thinking; refreshed at each turn boundary (c1470).
    model_manager: Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    /// Active in-flight binding for chrome; cleared when the run ends.
    active_turn: Arc<Mutex<Option<crate::agent::session::ActiveTurnBinding>>>,
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

fn prepare_turn_binding(
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    active_turn: &Arc<Mutex<Option<crate::agent::session::ActiveTurnBinding>>>,
    system_prompt: &Option<String>,
    run_model: &mut Option<(String, Arc<dyn XyModel>)>,
) -> Result<(Arc<dyn XyModel>, crate::protocol::ports::XyGenerateOptions), String> {
    let mm = model_manager.lock().unwrap_or_else(|e| e.into_inner());
    let meta = mm
        .current_model()
        .ok_or_else(|| "no model configured".to_string())?;
    let model_id = meta.id.clone();
    let thinking = mm.thinking_level();
    let levels = crate::agent::model::manager::ModelManager::levels_for_meta(meta);
    let binding = crate::agent::session::ActiveTurnBinding {
        model_id: meta.id.clone(),
        display_name: if meta.display_name.is_empty() {
            meta.id.clone()
        } else {
            meta.display_name.clone()
        },
        thinking,
        omit_thinking: !crate::protocol::types::ThinkingLevel::is_adjustable(&levels),
    };
    let generate_options = crate::protocol::ports::XyGenerateOptions {
        thinking_level: thinking,
        level_map: meta.thinking_level_map.clone(),
        thinking_budgets: None,
        system_prompt: system_prompt.clone(),
    };
    let model = match run_model.as_ref() {
        Some((id, model)) if id == &model_id => Arc::clone(model),
        _ => {
            let built = mm.build_current_model()?;
            *run_model = Some((model_id, Arc::clone(&built)));
            built
        }
    };
    drop(mm);
    *active_turn.lock().unwrap_or_else(|e| e.into_inner()) = Some(binding);
    Ok((model, generate_options))
}

/// Clears active-turn binding when the ReAct stream drops (normal end or abort).
struct ClearActiveTurn(Arc<Mutex<Option<crate::agent::session::ActiveTurnBinding>>>);

impl Drop for ClearActiveTurn {
    fn drop(&mut self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

fn should_stop_after_turn(hooks: &AgentHooks, ctx: &ShouldStopAfterTurnCtx) -> bool {
    hooks
        .should_stop_after_turn
        .as_ref()
        .is_some_and(|hook| hook(ctx))
}

fn drain_queue(queue: &Arc<Mutex<PendingMessageQueue>>) -> Vec<AgentMessage> {
    queue.lock().unwrap_or_else(|e| e.into_inner()).drain()
}

fn queue_counts(
    steer: &Arc<Mutex<PendingMessageQueue>>,
    follow_up: &Arc<Mutex<PendingMessageQueue>>,
) -> (usize, usize) {
    let steer_count = steer.lock().unwrap_or_else(|e| e.into_inner()).len();
    let follow_up_count = follow_up.lock().unwrap_or_else(|e| e.into_inner()).len();
    (steer_count, follow_up_count)
}

/// Turn-end compaction: Case1 overflow then Case2 threshold (c1640/c1660).
///
/// Returns `true` when overflow recovery asks the ReAct loop to continue
/// (compact succeeded with willRetry).
async fn try_turn_end_compaction(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    event_sink: &Arc<dyn crate::protocol::ports::XyEventSink>,
    settings: &crate::agent::compaction::CompactionSettings,
    history: &mut Vec<AgentMessage>,
    overflow_recovery_attempted: &mut bool,
) -> bool {
    use crate::agent::compaction::{CompactionOrchestrator, EstimateOpts, OverflowCompactOutcome};

    let last_assistant = history
        .iter()
        .rev()
        .find(|m| m.role_name() == "assistant")
        .cloned();
    let Some(last_assistant) = last_assistant else {
        return false;
    };

    let (model, ctx_window, model_id, provider) = {
        let mm = model_manager.lock().unwrap_or_else(|e| e.into_inner());
        let meta = match mm.current_model() {
            Some(m) => m,
            None => return false,
        };
        let ctx_window = meta.context_window;
        let model_id = meta.config.model.clone();
        let provider = meta.config.provider_name().to_string();
        let model = match mm.build_current_model() {
            Ok(m) => m,
            Err(e) => {
                log::warn!("turn-end compaction: no model: {e}");
                return false;
            }
        };
        (model, ctx_window, model_id, provider)
    };

    let orch = CompactionOrchestrator::new(settings.clone());

    match orch
        .maybe_overflow_compact(
            store.as_ref(),
            session_id,
            model.as_ref(),
            event_sink.as_ref(),
            ctx_window,
            &last_assistant,
            &provider,
            &model_id,
            *overflow_recovery_attempted,
        )
        .await
    {
        Ok(OverflowCompactOutcome::Ran { will_retry }) => {
            if will_retry {
                *overflow_recovery_attempted = true;
                // Strip trailing error assistant from working history (session keeps it).
                if matches!(
                    history.last(),
                    Some(AgentMessage::Llm(LlmMessage::AssistantMessage {
                        stop_reason: Some(crate::protocol::message::XyStopReason::Error),
                        ..
                    }))
                ) {
                    history.pop();
                }
                return true;
            }
            return false;
        }
        Ok(OverflowCompactOutcome::FailedOnce) => {
            return false;
        }
        Ok(OverflowCompactOutcome::Skipped) => {}
        Err(e) => {
            log::warn!("turn-end overflow compaction failed: {e}");
        }
    }

    let opts = EstimateOpts {
        model_id: Some(model_id),
        ..Default::default()
    };
    if let Err(e) = orch
        .maybe_auto_compact(
            store.as_ref(),
            session_id,
            model.as_ref(),
            event_sink.as_ref(),
            ctx_window,
            &opts,
            Some(&last_assistant),
        )
        .await
    {
        log::warn!("turn-end compaction failed: {e}");
    }
    false
}

async fn persist_agent_message(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    message: &AgentMessage,
) {
    let Ok(message) = serde_json::to_value(message) else {
        return;
    };
    let entry = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: String::new(),
        },
        message,
    });
    let _ = store.append_session_entry(session_id, &entry).await;
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
        let user_preview = parts_preview_text(&user_parts);
        let model_api = {
            let mm = model_manager.lock().unwrap_or_else(|e| e.into_inner());
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
                    // Bridge maps this to a dim system note + idle (not a sticky fault).
                    turn_aborted = true;
                    yield XyEvent::Error("aborted".to_string());
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
                    yield XyEvent::Error(format!("context hook blocked: {reason}"));
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
                        yield XyEvent::Error(format!("model build error: {e}"));
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
                        yield XyEvent::Error("aborted".to_string());
                        break 'outer;
                    }
                    Some(Ok(s)) => {
                        chunk_stream = s;
                    }
                    Some(Err(e)) => {
                        // c1660: overflow on connect → synthesize error assistant + Case1.
                        let err_text = e.clone();
                        let (provider, model_id) = {
                            let mm = model_manager.lock().unwrap_or_else(|err| err.into_inner());
                            mm.current_model()
                                .map(|m| {
                                    (
                                        m.config.provider_name().to_string(),
                                        m.config.model.clone(),
                                    )
                                })
                                .unwrap_or_default()
                        };
                        let err_asst = AgentMessage::Llm(LlmMessage::AssistantMessage {
                            content: vec![AgentPart::text("")],
                            stop_reason: Some(crate::protocol::message::XyStopReason::Error),
                            usage: None,
                            api: String::new(),
                            provider,
                            model: model_id,
                            response_id: None,
                            error_message: Some(err_text.clone()),
                            timestamp: crate::protocol::message::now_ms(),
                            diagnostics: Vec::new(),
                        });
                        persist_agent_message(&store, &session_id, &err_asst).await;
                        history.push(err_asst);
                        yield XyEvent::Error(e);
                        let will_continue = try_turn_end_compaction(
                            &store,
                            &session_id,
                            &model_manager,
                            &event_sink,
                            &compaction_settings,
                            &mut history,
                            &mut overflow_recovery_attempted,
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
                            let mut assistant_parts = Vec::new();
                            if !thinking_acc.is_empty() || thinking_signature.is_some() {
                                assistant_parts.push(AgentPart::Thinking {
                                    thinking: std::mem::take(&mut thinking_acc),
                                    redacted: false,
                                    thinking_signature: thinking_signature.take(),
                                });
                            }
                            if !text_acc.is_empty() {
                                assistant_parts.push(AgentPart::text(std::mem::take(
                                    &mut text_acc,
                                )));
                            }
                            for (id, name, args) in &tool_calls {
                                assistant_parts.push(AgentPart::ToolCall {
                                    id: id.clone(),
                                    name: name.clone(),
                                    arguments: args.clone(),
                                });
                            }
                            if !assistant_parts.is_empty() {
                                let assistant_msg =
                                    AgentMessage::Llm(LlmMessage::AssistantMessage {
                                        content: assistant_parts,
                                        stop_reason: Some(
                                            crate::protocol::message::XyStopReason::Aborted,
                                        ),
                                        usage: done_usage.take(),
                                        api: String::new(),
                                        provider: String::new(),
                                        model: String::new(),
                                        response_id: None,
                                        error_message: None,
                                        timestamp: crate::protocol::message::now_ms(),
                                        diagnostics: Vec::new(),
                                    });
                                yield XyEvent::MessageEnd {
                                    role: "assistant".to_string(),
                                    message: Some(assistant_msg.clone()),
                                };
                                persist_agent_message(&store, &session_id, &assistant_msg)
                                    .await;
                                history.push(assistant_msg);
                            }
                            turn_aborted = true;
                            yield XyEvent::Error("aborted".to_string());
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
                                yield XyEvent::MessageUpdate {
                                    text: text_acc.clone(),
                                    thinking: if thinking_acc.is_empty() {
                                        None
                                    } else {
                                        Some(thinking_acc.clone())
                                    },
                                    message: Some(partial_assistant_message(
                                        &text_acc,
                                        &thinking_acc,
                                        thinking_signature.as_deref(),
                                        &tool_calls,
                                    )),
                                };
                            }
                            XyChunk::ThinkingDelta(text) => {
                                thinking_acc.push_str(&text);
                                yield XyEvent::ThinkingDelta(text);
                                yield XyEvent::MessageUpdate {
                                    text: text_acc.clone(),
                                    thinking: Some(thinking_acc.clone()),
                                    message: Some(partial_assistant_message(
                                        &text_acc,
                                        &thinking_acc,
                                        thinking_signature.as_deref(),
                                        &tool_calls,
                                    )),
                                };
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
                                yield XyEvent::MessageUpdate {
                                    text: text_acc.clone(),
                                    thinking: if thinking_acc.is_empty() {
                                        None
                                    } else {
                                        Some(thinking_acc.clone())
                                    },
                                    message: Some(partial_assistant_message(
                                        &text_acc,
                                        &thinking_acc,
                                        thinking_signature.as_deref(),
                                        &tool_calls,
                                    )),
                                };
                            }
                            XyChunk::ToolCallStart { id, name } => {
                                upsert_streaming_tool(
                                    &mut tool_calls,
                                    id,
                                    name,
                                    Value::Object(Default::default()),
                                );
                                yield XyEvent::MessageUpdate {
                                    text: text_acc.clone(),
                                    thinking: if thinking_acc.is_empty() {
                                        None
                                    } else {
                                        Some(thinking_acc.clone())
                                    },
                                    message: Some(partial_assistant_message(
                                        &text_acc,
                                        &thinking_acc,
                                        thinking_signature.as_deref(),
                                        &tool_calls,
                                    )),
                                };
                            }
                            XyChunk::ToolCallDelta {
                                id,
                                name,
                                args,
                                ..
                            } => {
                                upsert_streaming_tool(&mut tool_calls, id, name, args);
                                yield XyEvent::MessageUpdate {
                                    text: text_acc.clone(),
                                    thinking: if thinking_acc.is_empty() {
                                        None
                                    } else {
                                        Some(thinking_acc.clone())
                                    },
                                    message: Some(partial_assistant_message(
                                        &text_acc,
                                        &thinking_acc,
                                        thinking_signature.as_deref(),
                                        &tool_calls,
                                    )),
                                };
                            }
                            XyChunk::ToolCallEnd { name, args, id } => {
                                // Intent only — execute after MessageEnd (c1255 / ar21).
                                upsert_streaming_tool(&mut tool_calls, id, name, args);
                                yield XyEvent::MessageUpdate {
                                    text: text_acc.clone(),
                                    thinking: if thinking_acc.is_empty() {
                                        None
                                    } else {
                                        Some(thinking_acc.clone())
                                    },
                                    message: Some(partial_assistant_message(
                                        &text_acc,
                                        &thinking_acc,
                                        thinking_signature.as_deref(),
                                        &tool_calls,
                                    )),
                                };
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
                            yield XyEvent::Error(err_text);
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

                let mut assistant_parts = Vec::new();
                if !thinking_acc.is_empty() || thinking_signature.is_some() {
                    assistant_parts.push(AgentPart::Thinking {
                        thinking: thinking_acc,
                        redacted: false,
                        thinking_signature,
                    });
                }
                if !text_acc.is_empty() {
                    assistant_parts.push(AgentPart::text(text_acc));
                }
                for (id, name, args) in &tool_calls {
                    assistant_parts.push(AgentPart::ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: args.clone(),
                    });
                }
                if !assistant_parts.is_empty()
                    || matches!(
                        done_stop_reason,
                        Some(crate::protocol::message::XyStopReason::Error)
                    )
                {
                    let (provider, model_id) = {
                        let mm = model_manager.lock().unwrap_or_else(|e| e.into_inner());
                        mm.current_model()
                            .map(|m| {
                                (
                                    m.config.provider_name().to_string(),
                                    m.config.model.clone(),
                                )
                            })
                            .unwrap_or_default()
                    };
                    if assistant_parts.is_empty() {
                        assistant_parts.push(AgentPart::text(""));
                    }
                    let assistant_msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
                        content: assistant_parts,
                        stop_reason: done_stop_reason,
                        usage: done_usage,
                        api: String::new(),
                        provider,
                        model: model_id,
                        response_id: None,
                        error_message: done_error_message,
                        timestamp: crate::protocol::message::now_ms(),
                        diagnostics: Vec::new(),
                    });
                    persist_agent_message(&store, &session_id, &assistant_msg).await;
                    history.push(assistant_msg);
                }

                continue_after_tools = !tool_calls.is_empty();

                if tool_calls.is_empty() {
                    let turn_index = turn as u32;
                    yield XyEvent::TurnEnd { turn_index };
                    if let Some(bus) = &hook_bus {
                        let (ty, phase, ctx) = super::script_hook_ctx::turn_end(turn as u32);
                        observe_script_hook(bus, ty, phase, ctx).await;
                    }
                    // c1640/c1660: pi `_checkCompaction` after settled assistant.
                    let will_continue = try_turn_end_compaction(
                        &store,
                        &session_id,
                        &model_manager,
                        &event_sink,
                        &compaction_settings,
                        &mut history,
                        &mut overflow_recovery_attempted,
                    )
                    .await;
                    if will_continue {
                        continue 'outer;
                    }
                    turn += 1;
                    let stop_ctx = ShouldStopAfterTurnCtx {
                        turn_index,
                        assistant: history
                            .iter()
                            .rev()
                            .find(|m| m.role_name() == "assistant")
                            .cloned(),
                        tool_results: Vec::new(),
                        history: history.clone(),
                        new_messages: history[run_baseline..].to_vec(),
                    };
                    if should_stop_after_turn(&hooks, &stop_ctx) {
                        // pi: agent_end without polling steer / follow-up.
                        break 'outer;
                    }
                    // Poll steering even when there were no tools (pi: pending
                    // after turn may restart the inner loop).
                    pending = drain_queue(&steer_queue);
                    if !pending.is_empty() {
                        let (steer_count, follow_up_count) =
                            queue_counts(&steer_queue, &follow_up_queue);
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

                let turn_index = turn as u32;
                yield XyEvent::TurnEnd { turn_index };
                if let Some(bus) = &hook_bus {
                    let (ty, phase, ctx) = super::script_hook_ctx::turn_end(turn as u32);
                    observe_script_hook(bus, ty, phase, ctx).await;
                }
                // c1640/c1660: pi `_checkCompaction` after settled assistant.
                let will_continue = try_turn_end_compaction(
                    &store,
                    &session_id,
                    &model_manager,
                    &event_sink,
                    &compaction_settings,
                    &mut history,
                    &mut overflow_recovery_attempted,
                )
                .await;
                if will_continue {
                    continue 'outer;
                }
                turn += 1;

                let stop_ctx = ShouldStopAfterTurnCtx {
                    turn_index,
                    assistant: turn_assistant,
                    tool_results: turn_tool_results,
                    history: history.clone(),
                    new_messages: history[run_baseline..].to_vec(),
                };
                if should_stop_after_turn(&hooks, &stop_ctx) {
                    // pi: agent_end without polling steer / follow-up.
                    break 'outer;
                }

                // After tools (or a text-only turn handled above), poll steering
                // for the next model round.
                pending = drain_queue(&steer_queue);
                if !pending.is_empty() {
                    let (steer_count, follow_up_count) =
                        queue_counts(&steer_queue, &follow_up_queue);
                    yield XyEvent::QueueUpdate {
                        steer_count,
                        follow_up_count,
                    };
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

/// UI / event preview for tool results — text notes only; never dump image base64.
fn parts_preview_text(parts: &[AgentPart]) -> String {
    let mut out = Vec::new();
    for part in parts {
        match part {
            AgentPart::Text { text } => out.push(text.clone()),
            AgentPart::Image(_) => out.push("[image]".into()),
            AgentPart::Thinking { thinking, .. } => out.push(thinking.clone()),
            AgentPart::ToolCall { name, .. } => out.push(format!("[toolCall:{name}]")),
        }
    }
    out.join("\n")
}

async fn observe_script_hook(
    bus: &Arc<dyn XyHookBus>,
    event_type: &str,
    phase: &str,
    context: serde_json::Value,
) {
    if let XyHookOutcome::Blocked { reason } = bus.dispatch(event_type, phase, context).await {
        log::warn!(
            "Script hook blocked observe-only lifecycle event (fail-open) event={} phase={} reason={}",
            event_type,
            phase,
            reason
        );
    }
}

/// Helper: call model with retry for transient errors.
async fn call_with_retry(
    model: &Arc<dyn XyModel>,
    messages: Vec<AgentMessage>,
    tool_schemas: &[XyToolSchema],
    retry_state: &RetryState,
    options: &crate::protocol::ports::XyGenerateOptions,
    turn_id: Option<&str>,
) -> Result<Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>, String> {
    let llm_messages = project_for_llm(&messages);
    loop {
        match model
            .generate_stream(llm_messages.clone(), tool_schemas, true, options.clone())
            .await
        {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                let err_msg = format!("model error: {e}");
                if is_retryable_error(&err_msg) && retry_state.can_retry() {
                    log::warn!(
                        target: "xylitol::react",
                        "model.generate_stream retrying error.kind={} turn_id={} error={e}",
                        e.kind(),
                        turn_id.unwrap_or("")
                    );
                    let delay = retry_state.next_delay();
                    retry_state.backoff(delay).await;
                    continue;
                }
                super::obs::record_xy_error("model.generate_stream", &e, turn_id);
                return Err(err_msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::agent::model::registry::ModelRegistry;
    use crate::agent::session::AgentCapabilities;
    use crate::infra::session::SessionManager;
    use crate::protocol::model_config::XyModelConfig;
    use crate::protocol::ports::{XyEventSink, XyModel, XySessionStore, XyStream};
    use crate::protocol::session::SessionEntry;
    use crate::protocol::types::XyModelMeta;

    type ModelBuilderFn =
        Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>;

    /// Model builder for tests — the real factory (tests register `Fake`/`OpenAi`
    /// model configs and rely on `build_provider` constructing the provider struct;
    /// no real network calls are made in unit assertions).
    fn fake_model_builder() -> ModelBuilderFn {
        Arc::new(crate::infra::provider::factory::build_provider)
    }

    #[tokio::test]
    async fn test_agent_session_builds_model() {
        let mut reg = ModelRegistry::new(std::sync::Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: crate::protocol::model_config::XyModelConfig {
                kind: crate::protocol::model_config::XyModelKind::OpenAi,
                api_key: "sk-test".into(),
                model: "mock-model".into(),
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

        let session_mgr = SessionManager::new(SessionManager::default_dir());
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr.clone());
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let mut session = AgentCapabilities::new(
            reg,
            ToolSet::from_iter(crate::infra::tools::default_tools()),
            store,
            sink,
            Some("You are helpful.".into()),
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            fake_model_builder(),
            crate::infra::permission::allow_all_permission(),
            Some(std::sync::Arc::new(
                crate::infra::bash_exec::InfraBashExecutor::new(),
            )),
            Some(std::sync::Arc::new(crate::infra::export::StdExportIo::new())),
            crate::agent::session::QueueMode::default(),
            crate::agent::session::QueueMode::default(),
            None,
        );
        session.select_model("mock").expect("select mock");

        assert!(session.current_model().is_some());
        assert_eq!(session.current_model().unwrap().id, "mock");
    }

    #[tokio::test]
    async fn test_agent_loop_emits_events() {
        let mut reg = ModelRegistry::new(std::sync::Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: crate::protocol::model_config::XyModelConfig {
                kind: crate::protocol::model_config::XyModelKind::OpenAi,
                api_key: "sk-test".into(),
                model: "mock-model".into(),
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

        let session_mgr = SessionManager::new(SessionManager::default_dir());
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr.clone());
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let session = select_mock(AgentCapabilities::new(
            reg,
            ToolSet::from_iter(crate::infra::tools::default_tools()),
            store,
            sink,
            Some("You are helpful.".into()),
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            fake_model_builder(),
            crate::infra::permission::allow_all_permission(),
            Some(std::sync::Arc::new(
                crate::infra::bash_exec::InfraBashExecutor::new(),
            )),
            Some(std::sync::Arc::new(crate::infra::export::StdExportIo::new())),
            crate::agent::session::QueueMode::default(),
            crate::agent::session::QueueMode::default(),
            None,
        ));

        let mut loop_runner = AgentRuntime::new(session);
        let _stream = loop_runner.run_with_id("hello", "test-session").await;
    }

    // ── Mock model / tool helpers for hook and snapshot tests ───────

    struct MockModel {
        chunks: Vec<crate::protocol::types::XyChunk>,
        /// Without max_iterations (c1430), a constant tool-call mock would loop
        /// forever. First `generate_stream` returns `chunks`; later calls stop.
        calls: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl XyModel for MockModel {
        fn name(&self) -> &str {
            "mock"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::types::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            use std::sync::atomic::Ordering;
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            let chunks = if n == 0 {
                self.chunks.clone()
            } else {
                vec![
                    crate::protocol::types::XyChunk::TextDelta("(mock end)".into()),
                    crate::protocol::types::XyChunk::Done {
                        finish_reason: crate::protocol::message::XyStopReason::Stop,
                        usage: None,
                    },
                ]
            };
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }

    struct MockTool;

    #[async_trait::async_trait]
    impl crate::protocol::ports::XyTool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }

        fn description(&self) -> &str {
            "mock"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            })
        }

        async fn execute(
            &self,
            _ctx: &crate::protocol::ports::XyToolCtx,
            _args: serde_json::Value,
        ) -> Result<String, crate::protocol::error::XyToolError> {
            Ok("executed".into())
        }
    }

    /// Emits multiple live output chunks before returning (bash-like uplink).
    struct StreamingMockTool;

    #[async_trait::async_trait]
    impl crate::protocol::ports::XyTool for StreamingMockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }

        fn description(&self) -> &str {
            "streaming mock"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}})
        }

        async fn execute(
            &self,
            ctx: &crate::protocol::ports::XyToolCtx,
            _args: serde_json::Value,
        ) -> Result<String, crate::protocol::error::XyToolError> {
            if let Some(tx) = &ctx.output_tx {
                for part in ["chunk-a\n", "chunk-b\n", "chunk-c\n"] {
                    let _ = tx.send(part.into()).await;
                    tokio::task::yield_now().await;
                }
            }
            Ok("done".into())
        }
    }

    fn mock_model_registry() -> ModelRegistry {
        let mut reg = ModelRegistry::new(std::sync::Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ));
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
        reg
    }

    fn select_mock(mut session: AgentCapabilities) -> AgentCapabilities {
        session
            .select_model("mock")
            .expect("select mock model for react tests");
        session
    }

    fn mock_model_builder(chunks: Vec<crate::protocol::types::XyChunk>) -> ModelBuilderFn {
        Arc::new(move |_| {
            Ok(Arc::new(MockModel {
                chunks: chunks.clone(),
                calls: std::sync::atomic::AtomicUsize::new(0),
            }) as Arc<dyn XyModel>)
        })
    }

    fn make_agent_with_tools(
        chunks: Vec<crate::protocol::types::XyChunk>,
        tools: ToolSet,
    ) -> AgentRuntime {
        make_agent_with_tools_and_store(chunks, tools).0
    }

    fn make_agent_with_tools_and_store(
        chunks: Vec<crate::protocol::types::XyChunk>,
        tools: ToolSet,
    ) -> (AgentRuntime, Arc<dyn XySessionStore>) {
        let reg = mock_model_registry();
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let mut session = AgentCapabilities::new(
            reg,
            tools,
            Arc::clone(&store),
            sink,
            None,
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            mock_model_builder(chunks),
            crate::infra::permission::allow_all_permission(),
            None,
            None,
            crate::agent::session::QueueMode::default(),
            crate::agent::session::QueueMode::default(),
            None,
        );
        session
            .select_model("mock")
            .expect("select mock model for react tests");
        (AgentRuntime::new(session), store)
    }

    #[tokio::test]
    async fn test_persist_done_usage() {
        use crate::protocol::message::{AgentMessage, LlmMessage, XyStopReason, XyUsage};
        use futures::StreamExt;

        let usage = XyUsage {
            input: 11,
            output: 7,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: 0,
            total_tokens: 18,
            cost: None,
        };
        let chunks = vec![
            crate::protocol::types::XyChunk::TextDelta("hi".into()),
            crate::protocol::types::XyChunk::Done {
                finish_reason: XyStopReason::Stop,
                usage: Some(usage),
            },
        ];
        let (mut agent, store) = make_agent_with_tools_and_store(chunks, ToolSet::from_iter([]));
        let mut stream = agent.run_with_id("ping", "sess-usage").await;
        while stream.next().await.is_some() {}

        let entries = store.load_entries("sess-usage").await.expect("entries");
        let mut found = false;
        for entry in entries {
            let SessionEntry::Message(m) = entry else {
                continue;
            };
            let Ok(AgentMessage::Llm(LlmMessage::AssistantMessage {
                usage: Some(u),
                stop_reason: Some(XyStopReason::Stop),
                ..
            })) = serde_json::from_value(m.message)
            else {
                continue;
            };
            assert_eq!(u.input, 11);
            assert_eq!(u.output, 7);
            found = true;
        }
        assert!(found, "expected persisted assistant with Done.usage");
    }

    #[tokio::test]
    async fn test_tool_intent_before_execution() {
        use crate::protocol::lifecycle::XyEvent;
        use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};
        use futures::StreamExt;

        let chunks = vec![
            crate::protocol::types::XyChunk::ToolCallStart {
                id: "call-1".into(),
                name: "mock_tool".into(),
            },
            crate::protocol::types::XyChunk::ToolCallDelta {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args_delta: r#"{"input":"#.into(),
                args: serde_json::json!({"input": ""}),
            },
            crate::protocol::types::XyChunk::ToolCallDelta {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args_delta: r#"x"}"#.into(),
                args: serde_json::json!({"input": "x"}),
            },
            crate::protocol::types::XyChunk::ToolCallEnd {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args: serde_json::json!({"input": "x"}),
            },
            crate::protocol::types::XyChunk::Done {
                finish_reason: crate::protocol::message::XyStopReason::ToolUse,
                usage: None,
            },
        ];
        let mut agent = make_agent_with_tools(
            chunks,
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
            ]),
        );

        let mut stream = agent.run("go").await;
        let mut saw_intent_update = false;
        let mut message_end_seen = false;
        let mut tool_start_after_end = false;
        let mut tool_update_seen = false;

        while let Some(evt) = stream.next().await {
            match evt {
                XyEvent::MessageUpdate {
                    message: Some(AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. })),
                    ..
                } if !message_end_seen
                    && content.iter().any(|p| {
                        matches!(
                            p,
                            AgentPart::ToolCall {
                                name,
                                ..
                            } if name == "mock_tool"
                        )
                    }) =>
                {
                    saw_intent_update = true;
                }
                XyEvent::MessageEnd { .. } => {
                    message_end_seen = true;
                    assert!(
                        saw_intent_update,
                        "expected MessageUpdate with ToolCall before MessageEnd"
                    );
                }
                XyEvent::ToolExecutionStart { .. } => {
                    assert!(
                        message_end_seen,
                        "ToolExecutionStart must not precede MessageEnd"
                    );
                    tool_start_after_end = true;
                }
                XyEvent::ToolExecutionUpdate { .. } => {
                    tool_update_seen = true;
                }
                _ => {}
            }
        }

        assert!(
            saw_intent_update,
            "expected streaming tool intent MessageUpdate"
        );
        assert!(
            tool_start_after_end,
            "expected ToolExecutionStart after MessageEnd"
        );
        assert!(
            tool_update_seen,
            "expected ToolExecutionUpdate during execution"
        );
    }

    #[tokio::test]
    async fn test_tool_execution_streams_multiple_updates() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![
            crate::protocol::types::XyChunk::ToolCallEnd {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args: serde_json::json!({}),
            },
            crate::protocol::types::XyChunk::Done {
                finish_reason: crate::protocol::message::XyStopReason::ToolUse,
                usage: None,
            },
        ];
        let mut agent = make_agent_with_tools(
            chunks,
            ToolSet::from_iter(vec![
                Arc::new(StreamingMockTool) as Arc<dyn crate::protocol::ports::XyTool>
            ]),
        );

        let mut stream = agent.run("go").await;
        let mut updates = Vec::new();
        let mut saw_end = false;
        while let Some(evt) = stream.next().await {
            match evt {
                XyEvent::ToolExecutionUpdate { output, .. } => updates.push(output),
                XyEvent::ToolExecutionEnd { .. } => saw_end = true,
                _ => {}
            }
        }
        assert!(
            updates.len() >= 3,
            "expected ≥3 live updates, got {updates:?}"
        );
        assert_eq!(updates[0], "chunk-a\n");
        assert_eq!(updates[1], "chunk-b\n");
        assert_eq!(updates[2], "chunk-c\n");
        assert!(saw_end, "expected ToolExecutionEnd");
    }

    #[tokio::test]
    async fn tool_execute_err_ends_with_tool_end_not_global_error() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        // Missing tool → ExecutionFailed; surfaces must not get a second XyEvent::Error.
        let chunks = vec![
            crate::protocol::types::XyChunk::ToolCallEnd {
                id: "call-missing".into(),
                name: "no_such_tool".into(),
                args: serde_json::json!({}),
            },
            crate::protocol::types::XyChunk::Done {
                finish_reason: crate::protocol::message::XyStopReason::ToolUse,
                usage: None,
            },
        ];
        let mut agent = make_agent_with_tools(chunks, ToolSet::empty());

        let mut stream = agent.run("go").await;
        let mut tool_ends = Vec::new();
        let mut global_errors = Vec::new();
        while let Some(evt) = stream.next().await {
            match evt {
                XyEvent::ToolExecutionEnd {
                    name,
                    result,
                    is_error,
                    ..
                } => tool_ends.push((name, result, is_error)),
                XyEvent::Error(msg) => global_errors.push(msg),
                _ => {}
            }
        }
        assert_eq!(
            tool_ends.len(),
            1,
            "expected one ToolExecutionEnd: {tool_ends:?}"
        );
        assert_eq!(tool_ends[0].0, "no_such_tool");
        assert!(tool_ends[0].2, "is_error");
        assert!(
            tool_ends[0].1.contains("Unknown tool"),
            "result: {}",
            tool_ends[0].1
        );
        assert!(
            global_errors.is_empty(),
            "tool failure must not also yield XyEvent::Error: {global_errors:?}"
        );
    }

    #[tokio::test]
    async fn test_before_hook_denies_tool_call() {
        use crate::agent::runtime::hooks::BeforeToolHook;
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::protocol::types::XyChunk::ToolCallEnd {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({"input": "x"}),
        }];
        let mut agent = make_agent_with_tools(
            chunks,
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
            ]),
        );

        let hook: BeforeToolHook = Arc::new(|name, _id, _args| {
            if name == "mock_tool" {
                Some("denied by test hook".into())
            } else {
                None
            }
        });
        agent.add_hook(hook);

        let mut stream = agent.run("go").await;
        let mut found = false;
        while let Some(evt) = stream.next().await {
            if let XyEvent::ToolExecutionEnd {
                name,
                result,
                is_error,
                id: _,
            } = evt
            {
                assert_eq!(name, "mock_tool");
                assert!(is_error);
                assert!(result.contains("denied by test hook"));
                found = true;
            }
        }
        assert!(found, "expected a denied tool execution event");
    }

    #[tokio::test]
    async fn test_after_hook_modifies_tool_result() {
        use crate::agent::runtime::hooks::AfterToolHook;
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::protocol::types::XyChunk::ToolCallEnd {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({"input": "x"}),
        }];
        let hooks = {
            let mut h = AgentHooks::empty();
            let after: AfterToolHook = Arc::new(|_name, _id, _result, _is_error| {
                Some((serde_json::Value::String("modified".into()), false))
            });
            h.add_after(after);
            h
        };

        let mut agent = make_agent_with_tools(
            chunks,
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
            ]),
        );
        agent.replace_hooks(hooks);

        let mut stream = agent.run("go").await;
        let mut found = false;
        while let Some(evt) = stream.next().await {
            if let XyEvent::ToolExecutionEnd {
                name,
                result,
                is_error,
                id: _,
            } = evt
            {
                assert_eq!(name, "mock_tool");
                assert!(!is_error);
                assert_eq!(result, "modified");
                found = true;
            }
        }
        assert!(found, "expected a modified tool execution event");
    }

    #[tokio::test]
    async fn test_set_tools_takes_effect_on_next_turn() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::protocol::types::XyChunk::ToolCallEnd {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({"input": "x"}),
        }];
        let mut agent = make_agent_with_tools(
            chunks.clone(),
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
            ]),
        );

        // First turn: mock_tool is available.
        let mut stream = agent.run("go").await;
        let mut first_turn_executed = false;
        while let Some(evt) = stream.next().await {
            if let XyEvent::ToolExecutionEnd {
                ref name, is_error, ..
            } = evt
            {
                assert_eq!(name, "mock_tool");
                assert!(!is_error);
                first_turn_executed = true;
            }
        }
        assert!(first_turn_executed);

        // Replace tools with an empty set between turns.
        agent.set_tools(ToolSet::empty());

        // Second turn: the loop still saw mock_tool in the original snapshot if
        // we had mutated it mid-stream, but because setters apply to the next
        // turn, this turn should report the tool as unknown.
        let mut stream = agent.run("go").await;
        let mut second_turn_error = false;
        while let Some(evt) = stream.next().await {
            if let XyEvent::ToolExecutionEnd {
                ref name, is_error, ..
            } = evt
            {
                assert_eq!(name, "mock_tool");
                assert!(is_error);
                second_turn_error = true;
            }
        }
        assert!(second_turn_error);
    }

    // ── Multi-round tool ReAct (真 bug 复现) ──────────────────────
    //
    // The bug: react.rs:455 `if done { break }` treated XyChunk::Done (which
    // just marks the end of ONE model stream) as the end of the whole turn.
    // Providers emit Done right after a FunctionCall (openai.rs:172), so a
    // tool-calling turn broke out of the for-loop after executing the tool,
    // and the model never got a continuation round with the tool result.
    // Correct ReAct: keep looping while the model calls tools; stop only when
    // a round produces NO tool call (pure text reply = done).

    /// A stateful mock that returns a DIFFERENT chunk sequence per model call,
    /// so a multi-round turn (tool call → text reply) can be exercised. Each
    /// `generate_stream` call pops the front sequence.
    struct StatefulMockModel {
        rounds: std::sync::Mutex<Vec<Vec<crate::protocol::types::XyChunk>>>,
    }
    #[async_trait::async_trait]
    impl XyModel for StatefulMockModel {
        fn name(&self) -> &str {
            "stateful-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::types::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let chunks = self.rounds.lock().unwrap().remove(0);
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }

    fn make_agent_with_rounds(
        rounds: Vec<Vec<crate::protocol::types::XyChunk>>,
        tools: ToolSet,
    ) -> AgentRuntime {
        use crate::protocol::ports::XyModelBuilder;
        let reg = mock_model_registry();
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let builder: XyModelBuilder = Arc::new(move |_| {
            Ok(Arc::new(StatefulMockModel {
                rounds: std::sync::Mutex::new(rounds.clone()),
            }) as Arc<dyn XyModel>)
        });
        let mut session = AgentCapabilities::new(
            reg,
            tools,
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            builder,
            crate::infra::permission::allow_all_permission(),
            None,
            None,
            crate::agent::session::QueueMode::default(),
            crate::agent::session::QueueMode::default(),
            None,
        );
        session
            .select_model("mock")
            .expect("select mock model for round tests");
        AgentRuntime::new(session)
    }

    #[tokio::test]
    async fn tool_call_then_continuation_round_reaches_final_text() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        // Round 1: model calls a tool. The provider appends Done after the
        // FunctionCall (openai.rs:156-176), so Done sets `done=true` — this is
        // exactly the case where the old `if done { break }` wrongly aborted.
        // Round 2: model gives the final text reply (no tool call) + Done.
        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![
            vec![
                crate::protocol::types::XyChunk::ToolCallEnd {
                    id: "call-1".into(),
                    name: "mock_tool".into(),
                    args: serde_json::json!({"input": "x"}),
                },
                done_stop(),
            ],
            vec![
                crate::protocol::types::XyChunk::TextDelta("the answer is 42".into()),
                done_stop(),
            ],
        ];
        let mut agent = make_agent_with_rounds(
            rounds,
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
            ]),
        );

        let mut stream = agent.run("go").await;
        let mut saw_tool = false;
        let mut saw_final_text = false;
        let mut turn_end_count = 0;
        while let Some(evt) = stream.next().await {
            match evt {
                XyEvent::ToolExecutionEnd { .. } => saw_tool = true,
                XyEvent::TextDelta(t) if t.contains("the answer is 42") => saw_final_text = true,
                XyEvent::TurnEnd { .. } => turn_end_count += 1,
                _ => {}
            }
        }
        assert!(saw_tool, "tool must execute in round 1");
        assert!(
            saw_final_text,
            "continuation text after the tool call MUST reach the caller (the bug dropped it)"
        );
        assert_eq!(
            turn_end_count, 2,
            "two ReAct iterations → two TurnEnd events"
        );
    }

    #[tokio::test]
    async fn steer_before_run_is_injected_into_history() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![vec![
            crate::protocol::types::XyChunk::TextDelta("ok".into()),
            done_stop(),
        ]];
        let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
        agent.steer("please be brief");

        let mut stream = agent.run("hello").await;
        let mut history = Vec::new();
        while let Some(evt) = stream.next().await {
            if let XyEvent::AgentEnd { messages } = evt {
                history = messages;
            }
        }
        let texts: Vec<String> = history
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(LlmMessage::UserMessage { content, .. }) => {
                    content.iter().find_map(|p| match p {
                        AgentPart::Text { text: t } => Some(t.clone()),
                        _ => None,
                    })
                }
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t == "please be brief"),
            "steering text must appear in history: {texts:?}"
        );
        assert!(texts.iter().any(|t| t == "hello"));
    }

    #[tokio::test]
    async fn follow_up_continues_after_text_only_turn() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![
            vec![
                crate::protocol::types::XyChunk::TextDelta("first".into()),
                done_stop(),
            ],
            vec![
                crate::protocol::types::XyChunk::TextDelta("second".into()),
                done_stop(),
            ],
        ];
        let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
        agent.follow_up("and also this");

        let mut stream = agent.run("start").await;
        let mut texts = Vec::new();
        let mut turn_ends = 0;
        while let Some(evt) = stream.next().await {
            match evt {
                XyEvent::TextDelta(t) => texts.push(t),
                XyEvent::TurnEnd { .. } => turn_ends += 1,
                _ => {}
            }
        }
        assert!(texts.iter().any(|t| t == "first"));
        assert!(
            texts.iter().any(|t| t == "second"),
            "follow-up must trigger a second model round: {texts:?}"
        );
        assert!(turn_ends >= 2);
    }

    #[tokio::test]
    async fn should_stop_after_turn_skips_follow_up_and_ends() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };
        // Two rounds available — stop hook must prevent the second.
        let rounds = vec![
            vec![
                crate::protocol::types::XyChunk::TextDelta("first".into()),
                done_stop(),
            ],
            vec![
                crate::protocol::types::XyChunk::TextDelta("second".into()),
                done_stop(),
            ],
        ];
        let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
        agent.follow_up("queued follow-up must stay");
        let calls = std::sync::Arc::new(AtomicUsize::new(0));
        let calls_hook = calls.clone();
        let seen_new = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_new_hook = seen_new.clone();
        agent.set_should_stop_after_turn(Some(std::sync::Arc::new(move |ctx| {
            calls_hook.fetch_add(1, Ordering::SeqCst);
            *seen_new_hook.lock().unwrap() = ctx.new_messages.clone();
            true
        })));

        let mut stream = agent.run("start").await;
        let mut texts = Vec::new();
        let mut turn_starts = 0u32;
        let mut turn_ends = 0u32;
        let mut agent_end_msgs = None;
        while let Some(evt) = stream.next().await {
            match evt {
                XyEvent::TextDelta(t) => texts.push(t),
                XyEvent::TurnStart { .. } => turn_starts += 1,
                XyEvent::TurnEnd { .. } => turn_ends += 1,
                XyEvent::AgentEnd { messages } => agent_end_msgs = Some(messages),
                _ => {}
            }
        }

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "stop hook once after TurnEnd"
        );
        assert_eq!(turn_starts, 1, "no second model turn");
        assert_eq!(turn_ends, 1);
        assert!(texts.iter().any(|t| t == "first"));
        assert!(
            !texts.iter().any(|t| t == "second"),
            "second model round must not run: {texts:?}"
        );
        assert_eq!(agent.queue_stats().follow_up_count, 1);
        let new_msgs = seen_new.lock().unwrap();
        let new_user: Vec<String> = new_msgs
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(LlmMessage::UserMessage { content, .. }) => {
                    content.iter().find_map(|p| match p {
                        AgentPart::Text { text: t } => Some(t.clone()),
                        _ => None,
                    })
                }
                _ => None,
            })
            .collect();
        assert!(
            new_user.iter().any(|t| t == "start"),
            "new_messages must carry the run prompt (pi newMessages): {new_user:?}"
        );
        let history = agent_end_msgs.expect("AgentEnd");
        let user_texts: Vec<String> = history
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(LlmMessage::UserMessage { content, .. }) => {
                    content.iter().find_map(|p| match p {
                        AgentPart::Text { text: t } => Some(t.clone()),
                        _ => None,
                    })
                }
                _ => None,
            })
            .collect();
        assert!(
            !user_texts.iter().any(|t| t.contains("queued follow-up")),
            "follow_up must not be injected: {user_texts:?}"
        );
    }

    #[tokio::test]
    async fn abort_before_run_does_not_stick_to_next_run() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![vec![
            crate::protocol::types::XyChunk::TextDelta("recovered".into()),
            done_stop(),
        ]];
        let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
        // Sticky-cancel bug: abort left the token cancelled forever.
        agent.abort();

        let mut stream = agent.run("hello").await;
        let mut texts = Vec::new();
        let mut aborted = false;
        while let Some(evt) = stream.next().await {
            match evt {
                XyEvent::TextDelta(t) => texts.push(t),
                XyEvent::Error(m) if m == "aborted" => aborted = true,
                _ => {}
            }
        }
        assert!(
            !aborted,
            "run() must install a fresh cancel token after abort"
        );
        assert_eq!(texts, vec!["recovered".to_string()]);
    }

    #[tokio::test]
    async fn abort_after_completed_run_allows_second_run() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;

        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };
        // Each `run` rebuilds the mock from the same round template — we only
        // assert the second run is not sticky-aborted (c482).
        let rounds = vec![vec![
            crate::protocol::types::XyChunk::TextDelta("ok".into()),
            done_stop(),
        ]];
        let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());

        let mut first = agent.run("1").await;
        while first.next().await.is_some() {}

        agent.abort();

        let mut second = agent.run("2").await;
        let mut texts = Vec::new();
        let mut aborted = false;
        while let Some(evt) = second.next().await {
            match evt {
                XyEvent::TextDelta(t) => texts.push(t),
                XyEvent::Error(m) if m == "aborted" => aborted = true,
                _ => {}
            }
        }
        assert!(!aborted, "second run must not immediately abort: {texts:?}");
        assert_eq!(texts, vec!["ok".to_string()]);
    }

    /// Mid-stream abort MUST stop polling the model stream (c680). Without
    /// `select!` on cancel inside the chunk loop, the consumer would drain all
    /// slow chunks even after `abort()` — proving UI-only abort is insufficient.
    #[tokio::test]
    async fn abort_mid_stream_stops_polling_model_chunks() {
        use crate::protocol::lifecycle::XyEvent;
        use futures::StreamExt;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct SlowMock {
            polled: Arc<AtomicUsize>,
        }

        #[async_trait::async_trait]
        impl XyModel for SlowMock {
            fn name(&self) -> &str {
                "slow-mock"
            }
            async fn generate_stream(
                &self,
                _messages: Vec<crate::protocol::message::LlmMessage>,
                _tools: &[crate::protocol::types::XyToolSchema],
                _stream: bool,
                _options: crate::protocol::ports::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                let polled = self.polled.clone();
                Ok(Box::pin(async_stream::stream! {
                    for i in 0..80u32 {
                        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                        polled.fetch_add(1, Ordering::SeqCst);
                        yield Ok(crate::protocol::types::XyChunk::TextDelta(format!("c{i}")));
                    }
                    yield Ok(crate::protocol::types::XyChunk::Done {
                        finish_reason: crate::protocol::message::XyStopReason::Stop,
                        usage: None,
                    });
                }))
            }
        }

        let polled = Arc::new(AtomicUsize::new(0));
        let polled_for_builder = polled.clone();
        let reg = mock_model_registry();
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
            Ok(Arc::new(SlowMock {
                polled: polled_for_builder.clone(),
            }) as Arc<dyn XyModel>)
        });
        let session = select_mock(AgentCapabilities::new(
            reg,
            ToolSet::empty(),
            store.clone(),
            sink,
            None,
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            builder,
            crate::infra::permission::allow_all_permission(),
            None,
            None,
            crate::agent::session::QueueMode::default(),
            crate::agent::session::QueueMode::default(),
            None,
        ));
        let mut agent = AgentRuntime::new(session);

        let mut stream = agent.run("go").await;
        let mut aborted = false;
        let mut text_count = 0usize;
        while let Some(evt) = stream.next().await {
            match evt {
                XyEvent::TextDelta(_) => {
                    text_count += 1;
                    if text_count == 1 {
                        agent.abort();
                    }
                }
                XyEvent::Error(m) if m == "aborted" => aborted = true,
                _ => {}
            }
        }

        assert!(aborted, "must surface aborted after mid-stream cancel");
        let n = polled.load(Ordering::SeqCst);
        assert!(
            n < 40,
            "abort must stop polling model chunks (polled={n}, text_count={text_count})"
        );
        assert!(
            text_count < 40,
            "UI/event consumer must not see a full drain after abort (text_count={text_count})"
        );

        // c1595: partial assistant persisted with stop_reason=Aborted.
        let sid = agent.inner().session_id().expect("session id after run");
        let entries = store.load_entries(sid).await.expect("load entries");
        let mut found_aborted = false;
        for e in &entries {
            let SessionEntry::Message(m) = e else {
                continue;
            };
            if m.message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
                continue;
            }
            if m.message.get("stopReason").and_then(|r| r.as_str()) == Some("aborted")
                || m.message.get("stop_reason").and_then(|r| r.as_str()) == Some("aborted")
            {
                found_aborted = true;
                break;
            }
        }
        assert!(
            found_aborted,
            "mid-stream abort must persist assistant with aborted stop_reason: {entries:?}"
        );
    }

    #[tokio::test]
    async fn persist_turn_writes_user_and_assistant_messages() {
        use futures::StreamExt;

        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![vec![
            crate::protocol::types::XyChunk::TextDelta("hello back".into()),
            done_stop(),
        ]];
        let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
        let sid = "persist-test-session".to_string();
        agent.inner_mut().set_session(sid.clone());
        let store = agent.session_store();

        let mut stream = agent.run_with_id("hello", &sid).await;
        while stream.next().await.is_some() {}

        let entries = store.load_entries(&sid).await.expect("load entries");
        let messages: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                SessionEntry::Message(m) => m.message.get("role").and_then(|r| r.as_str()),
                _ => None,
            })
            .collect();
        assert!(messages.contains(&"user"));
        assert!(messages.contains(&"assistant"));
    }

    #[tokio::test]
    async fn second_turn_model_input_includes_first_turn_messages() {
        use futures::StreamExt;

        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };

        struct RecordingMockModel {
            seen: std::sync::Arc<std::sync::Mutex<Vec<Vec<crate::protocol::message::LlmMessage>>>>,
            chunks: Vec<crate::protocol::types::XyChunk>,
        }

        #[async_trait::async_trait]
        impl XyModel for RecordingMockModel {
            fn name(&self) -> &str {
                "recording-mock"
            }

            async fn generate_stream(
                &self,
                messages: Vec<crate::protocol::message::LlmMessage>,
                _tools: &[crate::protocol::types::XyToolSchema],
                _stream: bool,
                _options: crate::protocol::ports::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                self.seen.lock().unwrap().push(messages);
                let chunks = self.chunks.clone();
                Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
            }
        }

        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let reg = mock_model_registry();
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let builder: ModelBuilderFn = {
            let seen = seen.clone();
            Arc::new(move |_| {
                Ok(Arc::new(RecordingMockModel {
                    seen: seen.clone(),
                    chunks: vec![
                        crate::protocol::types::XyChunk::TextDelta("ok".into()),
                        done_stop(),
                    ],
                }) as Arc<dyn XyModel>)
            })
        };
        let mut agent = AgentRuntime::new(select_mock(AgentCapabilities::new(
            reg,
            ToolSet::empty(),
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            builder,
            crate::infra::permission::allow_all_permission(),
            None,
            None,
            crate::agent::session::QueueMode::default(),
            crate::agent::session::QueueMode::default(),
            None,
        )));
        let sid = "multi-turn-session".to_string();
        agent.inner_mut().set_session(sid.clone());

        let mut first = agent.run_with_id("turn one", &sid).await;
        while first.next().await.is_some() {}

        let mut second = agent.run_with_id("turn two", &sid).await;
        while second.next().await.is_some() {}

        let rounds = seen.lock().unwrap();
        assert_eq!(rounds.len(), 2, "expected two model calls");
        let second_input = rounds.last().expect("second round");
        let user_texts: Vec<String> = second_input
            .iter()
            .filter_map(|m| match m {
                LlmMessage::UserMessage { content, .. } => content.iter().find_map(|p| match p {
                    AgentPart::Text { text: t } => Some(t.clone()),
                    _ => None,
                }),
                _ => None,
            })
            .collect();
        assert!(
            user_texts.iter().any(|t| t == "turn one"),
            "second turn must include first user prompt: {user_texts:?}"
        );
        assert!(user_texts.iter().any(|t| t == "turn two"));
    }

    #[tokio::test]
    async fn system_prompt_via_options_not_user_history() {
        use futures::StreamExt;

        struct RecordingMockModel {
            seen_msgs:
                std::sync::Arc<std::sync::Mutex<Vec<Vec<crate::protocol::message::LlmMessage>>>>,
            seen_opts:
                std::sync::Arc<std::sync::Mutex<Vec<crate::protocol::ports::XyGenerateOptions>>>,
        }

        #[async_trait::async_trait]
        impl XyModel for RecordingMockModel {
            fn name(&self) -> &str {
                "recording-mock"
            }

            async fn generate_stream(
                &self,
                messages: Vec<crate::protocol::message::LlmMessage>,
                _tools: &[crate::protocol::types::XyToolSchema],
                _stream: bool,
                options: crate::protocol::ports::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                self.seen_msgs.lock().unwrap().push(messages);
                self.seen_opts.lock().unwrap().push(options);
                Ok(Box::pin(futures::stream::iter(vec![Ok(
                    crate::protocol::types::XyChunk::Done {
                        finish_reason: crate::protocol::message::XyStopReason::Stop,
                        usage: None,
                    },
                )])))
            }
        }

        let seen_msgs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen_opts = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let reg = mock_model_registry();
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let builder: ModelBuilderFn = {
            let seen_msgs = seen_msgs.clone();
            let seen_opts = seen_opts.clone();
            Arc::new(move |_| {
                Ok(Arc::new(RecordingMockModel {
                    seen_msgs: seen_msgs.clone(),
                    seen_opts: seen_opts.clone(),
                }) as Arc<dyn XyModel>)
            })
        };
        let mut agent = AgentRuntime::new(select_mock(AgentCapabilities::new(
            reg,
            ToolSet::empty(),
            store,
            sink,
            Some("CUSTOM_SYSTEM_MARKER".into()),
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            builder,
            crate::infra::permission::allow_all_permission(),
            None,
            None,
            crate::agent::session::QueueMode::default(),
            crate::agent::session::QueueMode::default(),
            None,
        )));

        let mut stream = agent.run("real user hello").await;
        while stream.next().await.is_some() {}

        let opts = seen_opts.lock().unwrap();
        assert_eq!(opts.len(), 1);
        let sp = opts[0]
            .system_prompt
            .as_deref()
            .expect("system_prompt in options");
        assert!(
            sp.contains("CUSTOM_SYSTEM_MARKER"),
            "options must carry system: {sp}"
        );

        let rounds = seen_msgs.lock().unwrap();
        let first = &rounds[0];
        assert!(!first.is_empty(), "history must include user message");
        let first_text = match &first[0] {
            LlmMessage::UserMessage { content, .. } => content
                .iter()
                .find_map(|p| match p {
                    AgentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .unwrap_or(""),
            other => panic!("first history entry must be user, got {other:?}"),
        };
        assert_eq!(first_text, "real user hello");
        assert!(!first_text.contains("CUSTOM_SYSTEM_MARKER"));
    }

    #[tokio::test]
    async fn dollar_skill_expanded_for_model_history_stays_raw() {
        use crate::protocol::resource::SkillInfo;
        use crate::protocol::source_info::{SourceInfo, SourceOrigin, SourceScope};
        use futures::StreamExt;

        struct RecordingMockModel {
            seen: std::sync::Arc<std::sync::Mutex<Vec<Vec<crate::protocol::message::LlmMessage>>>>,
        }

        #[async_trait::async_trait]
        impl XyModel for RecordingMockModel {
            fn name(&self) -> &str {
                "recording-mock"
            }

            async fn generate_stream(
                &self,
                messages: Vec<crate::protocol::message::LlmMessage>,
                _tools: &[crate::protocol::types::XyToolSchema],
                _stream: bool,
                _options: crate::protocol::ports::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                self.seen.lock().unwrap().push(messages);
                Ok(Box::pin(futures::stream::iter(
                    [
                        crate::protocol::types::XyChunk::TextDelta("ok".into()),
                        crate::protocol::types::XyChunk::Done {
                            finish_reason: crate::protocol::message::XyStopReason::Stop,
                            usage: None,
                        },
                    ]
                    .into_iter()
                    .map(Ok),
                )))
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let skill_path = dir.path().join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: demo\ndescription: d\n---\n\nREACT_SKILL_BODY_MARKER\n",
        )
        .unwrap();

        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let reg = mock_model_registry();
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let builder: ModelBuilderFn = {
            let seen = seen.clone();
            Arc::new(move |_| {
                Ok(Arc::new(RecordingMockModel { seen: seen.clone() }) as Arc<dyn XyModel>)
            })
        };
        let mut agent = AgentRuntime::new(select_mock(AgentCapabilities::new(
            reg,
            ToolSet::empty(),
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            builder,
            crate::infra::permission::allow_all_permission(),
            None,
            None,
            crate::agent::session::QueueMode::default(),
            crate::agent::session::QueueMode::default(),
            None,
        )));
        agent.apply_skills(vec![SkillInfo {
            name: "demo".into(),
            description: Some("d".into()),
            source_info: SourceInfo {
                path: skill_path,
                source: "test".into(),
                scope: SourceScope::User,
                origin: SourceOrigin::TopLevel,
                base_dir: None,
            },
            disable_model_invocation: false,
        }]);

        let mut stream = agent.run("please use $demo and $nosuch").await;
        let mut history = Vec::new();
        while let Some(evt) = stream.next().await {
            if let XyEvent::AgentEnd { messages } = evt {
                history = messages;
            }
        }

        let rounds = seen.lock().unwrap();
        assert_eq!(rounds.len(), 1);
        let model_user: Vec<String> = rounds[0]
            .iter()
            .filter_map(|m| match m {
                LlmMessage::UserMessage { content, .. } => content.iter().find_map(|p| match p {
                    AgentPart::Text { text: t } => Some(t.clone()),
                    _ => None,
                }),
                _ => None,
            })
            .collect();
        assert!(
            model_user
                .iter()
                .any(|t| t.contains("REACT_SKILL_BODY_MARKER") && t.contains("$demo")),
            "model input must expand known $skill: {model_user:?}"
        );
        assert!(
            model_user.iter().any(|t| t.contains("$nosuch")),
            "unknown $ must pass through: {model_user:?}"
        );

        let hist_user: Vec<String> = history
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(LlmMessage::UserMessage { content, .. }) => {
                    content.iter().find_map(|p| match p {
                        AgentPart::Text { text: t } => Some(t.clone()),
                        _ => None,
                    })
                }
                _ => None,
            })
            .collect();
        assert!(
            hist_user
                .iter()
                .any(|t| t == "please use $demo and $nosuch"),
            "session history must stay raw: {hist_user:?}"
        );
        assert!(
            !hist_user
                .iter()
                .any(|t| t.contains("REACT_SKILL_BODY_MARKER")),
            "session history must not persist expanded body: {hist_user:?}"
        );
    }

    // ── c1545 tool batch (S1–S5 harness) ─────────────────────────────

    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    struct SlowTool {
        name: &'static str,
        mode: crate::protocol::ports::XyToolExecutionMode,
        sleep_ms: u64,
        /// Shared wall-clock log: (name, start_ms, end_ms) from a fixed epoch.
        log: Arc<Mutex<Vec<(String, u128, u128)>>>,
        epoch: Instant,
    }

    #[async_trait::async_trait]
    impl crate::protocol::ports::XyTool for SlowTool {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            "slow test tool"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}})
        }
        fn execution_mode(&self) -> crate::protocol::ports::XyToolExecutionMode {
            self.mode
        }
        async fn execute(
            &self,
            _ctx: &crate::protocol::ports::XyToolCtx,
            _args: serde_json::Value,
        ) -> Result<String, crate::protocol::error::XyToolError> {
            let start = self.epoch.elapsed().as_millis();
            tokio::time::sleep(Duration::from_millis(self.sleep_ms)).await;
            let end = self.epoch.elapsed().as_millis();
            self.log
                .lock()
                .unwrap()
                .push((self.name.to_string(), start, end));
            Ok(format!("{}-done", self.name))
        }
    }

    fn multi_tool_rounds(calls: Vec<(&str, &str)>) -> Vec<Vec<crate::protocol::types::XyChunk>> {
        let done_stop = || crate::protocol::types::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::Stop,
            usage: None,
        };
        let mut round1 = Vec::new();
        for (i, (name, args_json)) in calls.iter().enumerate() {
            let args: serde_json::Value =
                serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
            round1.push(crate::protocol::types::XyChunk::ToolCallEnd {
                id: format!("call-{i}"),
                name: (*name).into(),
                args,
            });
        }
        round1.push(done_stop());
        vec![
            round1,
            vec![
                crate::protocol::types::XyChunk::TextDelta("done".into()),
                done_stop(),
            ],
        ]
    }

    fn overlaps(a: (u128, u128), b: (u128, u128)) -> bool {
        a.0 < b.1 && b.0 < a.1
    }

    #[tokio::test]
    async fn batch_default_sequential_no_overlap() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let epoch = Instant::now();
        let tools = ToolSet::from_iter(vec![Arc::new(SlowTool {
            name: "slow_safe",
            mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 80,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>]);
        // Two calls to the same ParallelSafe tool — Sequential batch must not overlap.
        let rounds = multi_tool_rounds(vec![
            ("slow_safe", r#"{"n":1}"#),
            ("slow_safe", r#"{"n":2}"#),
        ]);
        let mut agent = make_agent_with_rounds(rounds, tools);
        agent.set_batch_mode(XyBatchMode::Sequential);
        let mut stream = agent.run("go").await;
        let mut ends = Vec::new();
        while let Some(ev) = stream.next().await {
            if let XyEvent::ToolExecutionEnd { id, .. } = ev {
                ends.push(id);
            }
        }
        assert_eq!(ends, vec!["call-0".to_string(), "call-1".to_string()]);
        let entries = log.lock().unwrap().clone();
        assert_eq!(entries.len(), 2);
        assert!(
            !overlaps((entries[0].1, entries[0].2), (entries[1].1, entries[1].2)),
            "Sequential must not overlap: {entries:?}"
        );
        assert!(
            entries[0].2 <= entries[1].1,
            "source-order serial: {entries:?}"
        );
    }

    #[tokio::test]
    async fn batch_barrier_parallel_overlap_then_barrier() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let epoch = Instant::now();
        let tools = ToolSet::from_iter(vec![
            Arc::new(SlowTool {
                name: "slow_safe",
                mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
                sleep_ms: 100,
                log: log.clone(),
                epoch,
            }) as Arc<dyn crate::protocol::ports::XyTool>,
            Arc::new(SlowTool {
                name: "slow_barrier",
                mode: crate::protocol::ports::XyToolExecutionMode::Sequential,
                sleep_ms: 50,
                log: log.clone(),
                epoch,
            }) as Arc<dyn crate::protocol::ports::XyTool>,
        ]);
        let rounds = multi_tool_rounds(vec![
            ("slow_safe", r#"{"n":1}"#),
            ("slow_safe", r#"{"n":2}"#),
            ("slow_barrier", r#"{}"#),
        ]);
        let mut agent = make_agent_with_rounds(rounds, tools);
        agent.set_batch_mode(XyBatchMode::BarrierParallel);
        let t0 = Instant::now();
        let mut stream = agent.run("go").await;
        while stream.next().await.is_some() {}
        let elapsed = t0.elapsed();
        let entries = log.lock().unwrap().clone();
        assert_eq!(entries.len(), 3, "{entries:?}");
        let by = |n: &str| {
            entries
                .iter()
                .find(|(name, _, _)| name == n)
                .cloned()
                .unwrap_or_else(|| panic!("missing {n} in {entries:?}"))
        };
        // Two slow_safe — find both by order of log push (start order may race).
        let safes: Vec<_> = entries
            .iter()
            .filter(|(n, _, _)| n == "slow_safe")
            .cloned()
            .collect();
        assert_eq!(safes.len(), 2);
        assert!(
            overlaps((safes[0].1, safes[0].2), (safes[1].1, safes[1].2)),
            "ParallelSafe window must overlap: {entries:?}"
        );
        let barrier = by("slow_barrier");
        let safe_end_max = safes.iter().map(|e| e.2).max().unwrap();
        assert!(
            safe_end_max <= barrier.1,
            "both safes must finish before barrier starts: {entries:?}"
        );
        // S3: wall clock ≪ 200ms serial (two 100ms safes).
        assert!(
            elapsed < Duration::from_millis(280),
            "expected parallel speedup, elapsed={elapsed:?}"
        );
    }

    #[tokio::test]
    async fn batch_barrier_preserves_source_windows() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let epoch = Instant::now();
        let tools = ToolSet::from_iter(vec![
            Arc::new(SlowTool {
                name: "slow_safe",
                mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
                sleep_ms: 60,
                log: log.clone(),
                epoch,
            }) as Arc<dyn crate::protocol::ports::XyTool>,
            Arc::new(SlowTool {
                name: "slow_barrier",
                mode: crate::protocol::ports::XyToolExecutionMode::Sequential,
                sleep_ms: 40,
                log: log.clone(),
                epoch,
            }) as Arc<dyn crate::protocol::ports::XyTool>,
        ]);
        // safe → barrier → safe : second safe must start after barrier ends.
        let rounds = multi_tool_rounds(vec![
            ("slow_safe", r#"{"n":1}"#),
            ("slow_barrier", r#"{}"#),
            ("slow_safe", r#"{"n":2}"#),
        ]);
        let mut agent = make_agent_with_rounds(rounds, tools);
        agent.set_batch_mode(XyBatchMode::BarrierParallel);
        let mut stream = agent.run("go").await;
        while stream.next().await.is_some() {}
        let entries = log.lock().unwrap().clone();
        assert_eq!(entries.len(), 3, "{entries:?}");
        // Log order = start order for serial barriers between safes.
        let first_safe = &entries[0];
        let barrier = entries
            .iter()
            .find(|(n, _, _)| n == "slow_barrier")
            .unwrap();
        let second_safe = entries
            .iter()
            .rev()
            .find(|(n, _, _)| n == "slow_safe")
            .unwrap();
        assert_eq!(first_safe.0, "slow_safe");
        assert!(
            first_safe.2 <= barrier.1,
            "first safe before barrier: {entries:?}"
        );
        assert!(
            barrier.2 <= second_safe.1,
            "second safe after barrier: {entries:?}"
        );
        assert!(
            !overlaps((first_safe.1, first_safe.2), (second_safe.1, second_safe.2)),
            "safes must not share a window across barrier: {entries:?}"
        );
    }

    #[tokio::test]
    async fn batch_mcp_never_parallel_even_if_trait_lies() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let epoch = Instant::now();
        let tools = ToolSet::from_iter(vec![
            Arc::new(SlowTool {
                name: "slow_safe",
                mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
                sleep_ms: 80,
                log: log.clone(),
                epoch,
            }) as Arc<dyn crate::protocol::ports::XyTool>,
            Arc::new(SlowTool {
                name: "mcp:fake:x",
                mode: crate::protocol::ports::XyToolExecutionMode::Parallel, // lie
                sleep_ms: 80,
                log: log.clone(),
                epoch,
            }) as Arc<dyn crate::protocol::ports::XyTool>,
        ]);
        let rounds = multi_tool_rounds(vec![
            ("slow_safe", r#"{"n":1}"#),
            ("mcp:fake:x", r#"{}"#),
            ("slow_safe", r#"{"n":2}"#),
        ]);
        let mut agent = make_agent_with_rounds(rounds, tools);
        agent.set_batch_mode(XyBatchMode::BarrierParallel);
        let mut stream = agent.run("go").await;
        while stream.next().await.is_some() {}
        let entries = log.lock().unwrap().clone();
        assert_eq!(entries.len(), 3, "{entries:?}");
        let mcp = entries.iter().find(|(n, _, _)| n == "mcp:fake:x").unwrap();
        for (n, s, e) in &entries {
            if n == "mcp:fake:x" {
                continue;
            }
            assert!(
                !overlaps((*s, *e), (mcp.1, mcp.2)),
                "mcp must not overlap with {n}: {entries:?}"
            );
        }
    }

    #[tokio::test]
    async fn batch_history_source_order_despite_completion_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let epoch = Instant::now();
        // First call sleeps longer so second finishes first under BarrierParallel.
        let tools = ToolSet::from_iter(vec![
            Arc::new(SlowTool {
                name: "slow_a",
                mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
                sleep_ms: 120,
                log: log.clone(),
                epoch,
            }) as Arc<dyn crate::protocol::ports::XyTool>,
            Arc::new(SlowTool {
                name: "slow_b",
                mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
                sleep_ms: 30,
                log: log.clone(),
                epoch,
            }) as Arc<dyn crate::protocol::ports::XyTool>,
        ]);
        let rounds = multi_tool_rounds(vec![("slow_a", r#"{}"#), ("slow_b", r#"{}"#)]);
        let mut agent = make_agent_with_rounds(rounds, tools);
        agent.set_batch_mode(XyBatchMode::BarrierParallel);
        let mut stream = agent.run("go").await;
        let mut history = Vec::new();
        let mut end_order = Vec::new();
        while let Some(ev) = stream.next().await {
            match ev {
                XyEvent::ToolExecutionEnd { id, .. } => end_order.push(id),
                XyEvent::AgentEnd { messages } => history = messages,
                _ => {}
            }
        }
        // End MAY be completion order (b before a).
        assert!(
            end_order.contains(&"call-0".to_string()) && end_order.contains(&"call-1".to_string())
        );
        let tool_results: Vec<_> = history
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(crate::protocol::message::LlmMessage::ToolResultMessage {
                    tool_use_id,
                    ..
                }) => Some(tool_use_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            tool_results,
            vec!["call-0".to_string(), "call-1".to_string()],
            "history toolResults must be source order; ends were {end_order:?}"
        );
    }
}
