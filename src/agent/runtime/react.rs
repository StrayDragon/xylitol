//! Agent execution loop — core ReAct loop with full event stream, hooks, and tool execution modes.
//!
//! NOTE: 本文件聚焦 ReAct 算法主体. 天花板: ~500 行 (含 inline tests), 因 run_react_loop
//! 的 async_stream 宏块是原子逻辑单元, 跨函数 yield 不可行. 升级: 当工具执行/流处理逻辑
//! 显著膨胀时, 考虑引入 sub-turn state machine 替代单宏块.

//!
//! Key features:
//! - ReAct loop with turn-based execution
//! - `AgentHooks`: before_tool_call, after_tool_call, transform_context
//! - Steering/follow-up via [`PendingMessageQueue`] on the session agent
//! - Per-tool execution modes: sequential / parallel
//! - Auto-retry on transient errors

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
    tool_calls: &[(String, String, Value)],
) -> AgentMessage {
    let mut parts = Vec::new();
    if !thinking.is_empty() {
        parts.push(AgentPart::thinking(thinking.to_string()));
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
        timestamp: crate::domain::message::now_ms(),
        diagnostics: Vec::new(),
    })
}

use super::permission_router::permission_target;
use super::retry::{RetryState, is_retryable_error};
use super::{AgentHooks, XyEvent, XyEventStream};
use crate::agent::prompt::expand_skills_in_agent_messages;
use crate::agent::session::{AgentCapabilities, PendingMessageQueue};
use crate::agent::tools::ToolSet;
use crate::domain::error::XyError;
use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};
use crate::domain::resource_types::SkillInfo;
use crate::domain::session_types::{EntryBase, MessageEntry, SessionEntry};
use crate::domain::types::{XyChunk, XyToolSchema};
use crate::runtime_protocol::{
    XyHookBus, XyHookOutcome, XyModel, XySessionStore, XyToolCtx, XyToolExecutionMode,
};

use crate::runtime_protocol::XyPermissionVerdict;

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
    /// [`crate::app::core::driver::Driver::abort`]).
    pub fn abort(&self) {
        self.cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cancel();
        self.inner.clear_steer_queue();
        self.inner.abort_bash();
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

    /// Session store shared with the Driver seam.
    pub fn session_store(&self) -> Arc<dyn crate::runtime_protocol::XySessionStore> {
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

    /// Set the permission port. Takes effect on the next [`run`](Self::run) call.
    pub fn set_permission(&mut self, permission: Arc<dyn crate::runtime_protocol::XyPermission>) {
        self.inner.set_permission(permission);
    }

    /// Set the tool execution mode. Takes effect on the next [`run`](Self::run) call.
    pub fn set_tool_mode(&mut self, mode: crate::runtime_protocol::XyToolExecutionMode) {
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
    pub fn apply_skills(&mut self, skills: Vec<crate::domain::resource_types::SkillInfo>) {
        self.inner.apply_skills(skills);
    }

    /// Names currently injected into the system prompt (c1085).
    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.inner.loaded_skill_names()
    }

    /// Full skill catalog for `$` completion / expand (c1130).
    pub fn loaded_skills(&self) -> &[crate::domain::resource_types::SkillInfo] {
        self.inner.loaded_skills()
    }

    /// Run a turn with an auto-generated session_id.
    pub async fn run(&mut self, prompt: &str) -> XyEventStream {
        self.run_parts_with_id(
            vec![crate::domain::message::AgentPart::text(prompt)],
            &uuid::Uuid::new_v4().to_string(),
        )
        .await
    }

    /// Run a multi-part user turn (text + images, c1155).
    pub async fn run_parts(
        &mut self,
        parts: Vec<crate::domain::message::AgentPart>,
    ) -> XyEventStream {
        self.run_parts_with_id(parts, &uuid::Uuid::new_v4().to_string())
            .await
    }

    /// Run a turn with an explicit session_id.
    #[allow(clippy::type_complexity)]
    pub async fn run_with_id(&mut self, prompt: &str, session_id: &str) -> XyEventStream {
        self.run_parts_with_id(
            vec![crate::domain::message::AgentPart::text(prompt)],
            session_id,
        )
        .await
    }

    /// Run a multi-part user turn with an explicit session_id (c1155).
    #[allow(clippy::type_complexity)]
    pub async fn run_parts_with_id(
        &mut self,
        parts: Vec<crate::domain::message::AgentPart>,
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

        let model = match self.inner.build_current_model() {
            Ok(m) => m,
            Err(e) => return XyEventStream::error(format!("model build error: {e}")),
        };

        let tools = self.inner.tools().clone();
        let max_iterations = self.inner.max_iterations();
        let system_prompt = self.inner.system_prompt().map(|s| s.to_string());
        let hooks = self.inner.hooks().clone();
        let hook_bus = self.inner.hook_bus();
        let tool_mode = self.inner.tool_mode();
        let user_parts = parts;

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
        let generate_options = crate::runtime_protocol::XyGenerateOptions {
            thinking_level: self.inner.thinking_level(),
            level_map: self
                .inner
                .current_model()
                .map(|m| m.thinking_level_map.clone())
                .unwrap_or_default(),
            thinking_budgets: None,
        };

        let (queue_tx, mut queue_rx) = tokio::sync::mpsc::unbounded_channel();
        queues.bind_event_tx(queue_tx);

        let react = Box::pin(run_react_loop(ReActConfig {
            model,
            tools,
            tool_schemas,
            system_prompt,
            max_iterations: max_iterations as usize,
            user_parts,
            cancel,
            permission_check,
            hooks,
            hook_bus,
            tool_mode,
            steer_queue,
            follow_up_queue,
            store,
            session_id: sid,
            seeded_history,
            skills,
            generate_options,
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
    model: Arc<dyn XyModel>,
    tools: ToolSet,
    tool_schemas: Vec<XyToolSchema>,
    system_prompt: Option<String>,
    max_iterations: usize,
    user_parts: Vec<crate::domain::message::AgentPart>,
    cancel: CancellationToken,
    /// Optional permission check. Called with (tool_name, target_path_or_domain).
    /// Returns Some(reason) if the operation is denied.
    #[allow(clippy::type_complexity)]
    permission_check: Option<std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>>,
    /// Hooks consulted at tool-call boundaries.
    hooks: AgentHooks,
    /// Optional script hook bus (pi-aligned lifecycle + tool/context bridge).
    hook_bus: Option<Arc<dyn XyHookBus>>,
    /// Tool execution mode (currently advisory; sequential execution is the
    /// conservative default).
    tool_mode: XyToolExecutionMode,
    steer_queue: Arc<Mutex<PendingMessageQueue>>,
    follow_up_queue: Arc<Mutex<PendingMessageQueue>>,
    store: Arc<dyn XySessionStore>,
    session_id: String,
    seeded_history: Vec<AgentMessage>,
    /// Trust-filtered catalog for `$skill` expand (c1130); clone kept raw in history.
    skills: Vec<SkillInfo>,
    /// Thinking level / map / budgets for provider request assembly (c1165).
    generate_options: crate::runtime_protocol::XyGenerateOptions,
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
        model,
        tools,
        tool_schemas,
        system_prompt,
        max_iterations,
        user_parts,
        cancel,
        permission_check,
        hooks,
        hook_bus,
        tool_mode: _tool_mode,
        steer_queue,
        follow_up_queue,
        store,
        session_id,
        seeded_history,
        skills,
        generate_options,
    } = cfg;
    async_stream::stream! {
        if let Some(bus) = &hook_bus {
            observe_script_hook(bus, "agent_start", "", serde_json::json!({})).await;
        }

        let mut history: Vec<AgentMessage> = seeded_history;

        // First turn only: prepend configured system prompt when store is empty.
        if history.is_empty()
            && let Some(ref sp) = system_prompt
        {
            history.push(AgentMessage::user(sp.clone()));
        }

        // Add user message (text and/or images, c1155).
        history.push(AgentMessage::user_parts(user_parts));
        persist_agent_message(&store, &session_id, history.last().expect("user message")).await;

        let retry_state = RetryState::new(3, 1000);
        // Steering queued before/at run start is injected before the first model call.
        let mut pending: Vec<AgentMessage> = drain_queue(&steer_queue);
        let mut turn: usize = 0;

        // Outer loop: continues when follow-up messages arrive after the agent
        // would otherwise stop (pi runLoop semantics).
        'outer: loop {
            let mut continue_after_tools = true;

            while continue_after_tools || !pending.is_empty() {
                if cancel.is_cancelled() {
                    // Bridge maps this to a dim system note + idle (not a sticky fault).
                    yield XyEvent::Error("aborted".to_string());
                    break 'outer;
                }
                if turn >= max_iterations {
                    break 'outer;
                }

                yield XyEvent::TurnStart { turn_index: turn as u32 };
                if let Some(bus) = &hook_bus {
                    observe_script_hook(
                        bus,
                        "turn_start",
                        "",
                        serde_json::json!({ "turn_index": turn }),
                    )
                    .await;
                }

                // Inject pending messages (steering / follow-up) before the model call.
                // Emit user MessageStart/End so surfaces can 上行 scrollback (pi chat).
                if !pending.is_empty() {
                    for message in pending.drain(..) {
                        yield XyEvent::MessageStart {
                            role: "user".to_string(),
                            message: Some(message.clone()),
                        };
                        if let Some(bus) = &hook_bus {
                            observe_script_hook(
                                bus,
                                "message_start",
                                "",
                                serde_json::json!({ "role": "user" }),
                            )
                            .await;
                        }
                        yield XyEvent::MessageEnd {
                            role: "user".to_string(),
                            message: Some(message.clone()),
                        };
                        if let Some(bus) = &hook_bus {
                            observe_script_hook(
                                bus,
                                "message_end",
                                "",
                                serde_json::json!({ "role": "user" }),
                            )
                            .await;
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
                    && let XyHookOutcome::Blocked { reason } = bus
                        .dispatch(
                            "context",
                            "pre",
                            serde_json::json!({ "message_count": messages.len() }),
                        )
                        .await
                {
                    yield XyEvent::Error(format!("context hook blocked: {reason}"));
                    break 'outer;
                }

                // Race cancel against connect/retry so Esc aborts hung `send()`
                // (reqwest drop-cancels the in-flight HTTP future).
                let stream_result = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => None,
                    result = call_with_retry(
                        &model, messages, &tool_schemas, &retry_state, &generate_options,
                    ) => Some(result),
                };

                let mut chunk_stream: Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;
                match stream_result {
                    None => {
                        yield XyEvent::Error("aborted".to_string());
                        break 'outer;
                    }
                    Some(Ok(s)) => chunk_stream = s,
                    Some(Err(e)) => {
                        yield XyEvent::Error(e);
                        break 'outer;
                    }
                }

                yield XyEvent::MessageStart {
                    role: "assistant".to_string(),
                    message: None,
                };
                if let Some(bus) = &hook_bus {
                    observe_script_hook(
                        bus,
                        "message_start",
                        "",
                        serde_json::json!({ "role": "assistant" }),
                    )
                    .await;
                }

                let mut text_acc = String::new();
                let mut thinking_acc = String::new();
                let mut tool_calls: Vec<(String, String, Value)> = Vec::new();

                // Mid-stream abort: drop `chunk_stream` so adapter/reqwest closes
                // the HTTP body (c680). Surfaces inherit via Driver::abort → token.
                loop {
                    let chunk_result = tokio::select! {
                        biased;
                        _ = cancel.cancelled() => {
                            drop(chunk_stream);
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
                                        &tool_calls,
                                    )),
                                };
                            }
                            XyChunk::Done { .. } => {
                                // Stream-end marker for this single model call.
                            }
                        },
                        Some(Err(e)) => {
                            yield XyEvent::Error(format!("stream error: {e}"));
                            break;
                        }
                    }
                }

                let assistant_partial = if text_acc.is_empty()
                    && thinking_acc.is_empty()
                    && tool_calls.is_empty()
                {
                    None
                } else {
                    Some(partial_assistant_message(
                        &text_acc,
                        &thinking_acc,
                        &tool_calls,
                    ))
                };
                yield XyEvent::MessageEnd {
                    role: "assistant".to_string(),
                    message: assistant_partial,
                };
                if let Some(bus) = &hook_bus {
                    observe_script_hook(
                        bus,
                        "message_end",
                        "",
                        serde_json::json!({ "role": "assistant" }),
                    )
                    .await;
                }

                let mut assistant_parts = Vec::new();
                if !thinking_acc.is_empty() {
                    assistant_parts.push(AgentPart::thinking(thinking_acc));
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
                if !assistant_parts.is_empty() {
                    let assistant_msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
                        content: assistant_parts,
                        stop_reason: None,
                        usage: None,
                        api: String::new(),
                        provider: String::new(),
                        model: String::new(),
                        response_id: None,
                        error_message: None,
                        timestamp: crate::domain::message::now_ms(),
                        diagnostics: Vec::new(),
                    });
                    persist_agent_message(&store, &session_id, &assistant_msg).await;
                    history.push(assistant_msg);
                }

                continue_after_tools = !tool_calls.is_empty();

                if tool_calls.is_empty() {
                    yield XyEvent::TurnEnd { turn_index: turn as u32 };
                    if let Some(bus) = &hook_bus {
                        observe_script_hook(
                            bus,
                            "turn_end",
                            "",
                            serde_json::json!({ "turn_index": turn }),
                        )
                        .await;
                    }
                    turn += 1;
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

                for (id, name, args) in &tool_calls {
                    yield XyEvent::ToolExecutionStart {
                        id: id.clone(),
                        name: name.clone(),
                        args: args.clone(),
                    };

                    let tool = tools.get(name);
                    let tool_missing = tool.is_none();
                    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<String>(64);
                    let ctx = XyToolCtx::with_cancel(id, cancel.clone()).with_output_tx(out_tx);
                    let mut tool_args = args.clone();

                    let mut denied_reason: Option<String> = None;
                    if let Some(bus) = &hook_bus {
                        match bus
                            .dispatch(
                                "tool_call",
                                "pre",
                                serde_json::json!({ "tool": name, "args": tool_args }),
                            )
                            .await
                        {
                            XyHookOutcome::Blocked { reason } => {
                                denied_reason = Some(reason);
                            }
                            XyHookOutcome::Modified { args: modified } => {
                                tool_args = modified;
                            }
                            XyHookOutcome::Allowed => {}
                        }
                    }
                    if !hooks.before_tool_call.is_empty() {
                        for hook in &hooks.before_tool_call {
                            if let Some(reason) = hook(name, id, &tool_args) {
                                denied_reason = Some(reason);
                                break;
                            }
                        }
                    }

                    if denied_reason.is_none()
                        && let Some(ref check) = permission_check
                    {
                        let target = permission_target(name, &tool_args);
                        if let Some(reason) = check(name, &target) {
                            denied_reason = Some(format!("permission denied: {reason}"));
                        }
                    }

                    if let Some(reason) = denied_reason {
                        let err = format!("Tool '{name}' blocked: {reason}");
                        yield XyEvent::ToolExecutionUpdate {
                            id: id.clone(),
                            output: err.clone(),
                        };
                        yield XyEvent::ToolExecutionEnd {
                            id: id.clone(),
                            name: name.clone(),
                            result: err.clone(),
                            is_error: true,
                        };
                        history.push(AgentMessage::tool_result(
                            id.clone(),
                            name.clone(),
                            vec![AgentPart::text(err.clone())],
                            true,
                        ));
                        persist_agent_message(
                            &store,
                            &session_id,
                            history.last().expect("tool result"),
                        )
                        .await;
                        continue;
                    }

                    // Race tool future against live output chunks (bash uplink).
                    let exec_fut = async {
                        match tool {
                            Some(t) => t.execute_as_parts(&ctx, tool_args.clone()).await,
                            None => Err(crate::domain::error::XyToolError::ExecutionFailed(
                                anyhow::anyhow!("Unknown tool: {name}"),
                            )),
                        }
                    };
                    tokio::pin!(exec_fut);
                    let mut streamed_output = false;
                    let exec_outcome = loop {
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => {
                                break Err(crate::domain::error::XyToolError::Aborted);
                            }
                            chunk = out_rx.recv() => {
                                match chunk {
                                    Some(output) => {
                                        streamed_output = true;
                                        yield XyEvent::ToolExecutionUpdate {
                                            id: id.clone(),
                                            output,
                                        };
                                    }
                                    None => {
                                        // Sender dropped — wait for execute to finish.
                                        break exec_fut.await;
                                    }
                                }
                            }
                            done = &mut exec_fut => {
                                break done;
                            }
                        }
                    };
                    // Drain any chunks that arrived after the future completed.
                    while let Ok(output) = out_rx.try_recv() {
                        streamed_output = true;
                        yield XyEvent::ToolExecutionUpdate {
                            id: id.clone(),
                            output,
                        };
                    }

                    let mut result = match exec_outcome {
                        Ok(parts) => (parts, false),
                        Err(crate::domain::error::XyToolError::Aborted) => {
                            let err = format!("Tool '{name}' aborted");
                            (vec![AgentPart::text(err)], true)
                        }
                        Err(e) => {
                            let err = if tool_missing {
                                format!("Unknown tool: {name}")
                            } else {
                                format!("Tool '{name}' error: {e}")
                            };
                            yield XyEvent::Error(err.clone());
                            (vec![AgentPart::text(err)], true)
                        }
                    };

                    if !hooks.after_tool_call.is_empty() {
                        // Hooks still see a string/JSON value (text preview); Image parts are
                        // preserved unless the hook replaces the whole result with text.
                        let mut hook_value = serde_json::Value::String(parts_preview_text(&result.0));
                        let mut hook_err = result.1;
                        for hook in &hooks.after_tool_call {
                            if let Some((new_value, new_is_error)) =
                                hook(name, id, hook_value.clone(), hook_err)
                            {
                                hook_value = new_value;
                                hook_err = new_is_error;
                                result = (
                                    vec![AgentPart::text(match hook_value {
                                        serde_json::Value::String(ref s) => s.clone(),
                                        ref other => other.to_string(),
                                    })],
                                    hook_err,
                                );
                            }
                        }
                        result.1 = hook_err;
                    }
                    if let Some(bus) = &hook_bus
                        && let XyHookOutcome::Modified { args: modified } = bus
                            .dispatch(
                                "tool_result",
                                "post",
                                serde_json::json!({
                                    "tool": name,
                                    "result": parts_preview_text(&result.0),
                                    "is_error": result.1,
                                }),
                            )
                            .await
                    {
                        if let Some(val) = modified.get("result") {
                            let text = match val {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            result.0 = vec![AgentPart::text(text)];
                        }
                        if let Some(err) = modified.get("is_error").and_then(|v| v.as_bool()) {
                            result.1 = err;
                        }
                    }

                    let result_text = parts_preview_text(&result.0);

                    // Avoid appending the final JSON blob on top of live bash chunks.
                    if !streamed_output {
                        yield XyEvent::ToolExecutionUpdate {
                            id: id.clone(),
                            output: result_text.clone(),
                        };
                    }
                    yield XyEvent::ToolExecutionEnd {
                        id: id.clone(),
                        name: name.clone(),
                        result: result_text.clone(),
                        is_error: result.1,
                    };

                    history.push(AgentMessage::tool_result(
                        id.clone(),
                        name.clone(),
                        result.0,
                        result.1,
                    ));
                    persist_agent_message(
                        &store,
                        &session_id,
                        history.last().expect("tool result"),
                    )
                    .await;
                }

                yield XyEvent::TurnEnd { turn_index: turn as u32 };
                if let Some(bus) = &hook_bus {
                    observe_script_hook(
                        bus,
                        "turn_end",
                        "",
                        serde_json::json!({ "turn_index": turn }),
                    )
                    .await;
                }
                turn += 1;

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
            observe_script_hook(bus, "agent_settled", "", serde_json::json!({})).await;
            observe_script_hook(bus, "agent_end", "", serde_json::json!({})).await;
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
    options: &crate::runtime_protocol::XyGenerateOptions,
) -> Result<Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>, String> {
    loop {
        match model
            .generate_stream(messages.clone(), tool_schemas, true, options.clone())
            .await
        {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                let err_msg = format!("model error: {e}");
                if is_retryable_error(&err_msg) && retry_state.can_retry() {
                    let delay = retry_state.next_delay();
                    retry_state.backoff(delay).await;
                    continue;
                }
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
    use crate::domain::model::XyModelConfig;
    use crate::domain::session_types::SessionEntry;
    use crate::domain::types::XyModelMeta;
    use crate::infra::session::SessionManager;
    use crate::runtime_protocol::{XyEventSink, XyModel, XySessionStore, XyStream};

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
            config: crate::domain::model::XyModelConfig {
                kind: crate::domain::model::XyModelKind::OpenAi,
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
        let session = AgentCapabilities::new(
            reg,
            ToolSet::from_iter(crate::infra::tools::default_tools()),
            store,
            sink,
            Some("You are helpful.".into()),
            Vec::new(),
            Vec::new(),
            50,
            0.8,
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
            config: crate::domain::model::XyModelConfig {
                kind: crate::domain::model::XyModelKind::OpenAi,
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
        let session = AgentCapabilities::new(
            reg,
            ToolSet::from_iter(crate::infra::tools::default_tools()),
            store,
            sink,
            Some("You are helpful.".into()),
            Vec::new(),
            Vec::new(),
            50,
            0.8,
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

        let mut loop_runner = AgentRuntime::new(session);
        let _stream = loop_runner.run_with_id("hello", "test-session").await;
    }

    // ── Mock model / tool helpers for hook and snapshot tests ───────

    struct MockModel {
        chunks: Vec<crate::domain::types::XyChunk>,
    }

    #[async_trait::async_trait]
    impl XyModel for MockModel {
        fn name(&self) -> &str {
            "mock"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<AgentMessage>,
            _tools: &[crate::domain::types::XyToolSchema],
            _stream: bool,
            _options: crate::runtime_protocol::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let chunks = self.chunks.clone();
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }

    struct MockTool;

    #[async_trait::async_trait]
    impl crate::runtime_protocol::XyTool for MockTool {
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
            _ctx: &crate::runtime_protocol::XyToolCtx,
            _args: serde_json::Value,
        ) -> Result<String, crate::domain::error::XyToolError> {
            Ok("executed".into())
        }
    }

    /// Emits multiple live output chunks before returning (bash-like uplink).
    struct StreamingMockTool;

    #[async_trait::async_trait]
    impl crate::runtime_protocol::XyTool for StreamingMockTool {
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
            ctx: &crate::runtime_protocol::XyToolCtx,
            _args: serde_json::Value,
        ) -> Result<String, crate::domain::error::XyToolError> {
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
                kind: crate::domain::model::XyModelKind::Fake,
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

    fn mock_model_builder(chunks: Vec<crate::domain::types::XyChunk>) -> ModelBuilderFn {
        Arc::new(move |_| {
            Ok(Arc::new(MockModel {
                chunks: chunks.clone(),
            }) as Arc<dyn XyModel>)
        })
    }

    fn make_agent_with_tools(
        chunks: Vec<crate::domain::types::XyChunk>,
        tools: ToolSet,
    ) -> AgentRuntime {
        let reg = mock_model_registry();
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let session = AgentCapabilities::new(
            reg,
            tools,
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            50,
            0.8,
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
        AgentRuntime::new(session)
    }

    #[tokio::test]
    async fn test_tool_intent_before_execution() {
        use crate::domain::lifecycle::XyEvent;
        use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};
        use futures::StreamExt;

        let chunks = vec![
            crate::domain::types::XyChunk::ToolCallStart {
                id: "call-1".into(),
                name: "mock_tool".into(),
            },
            crate::domain::types::XyChunk::ToolCallDelta {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args_delta: r#"{"input":"#.into(),
                args: serde_json::json!({"input": ""}),
            },
            crate::domain::types::XyChunk::ToolCallDelta {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args_delta: r#"x"}"#.into(),
                args: serde_json::json!({"input": "x"}),
            },
            crate::domain::types::XyChunk::ToolCallEnd {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args: serde_json::json!({"input": "x"}),
            },
            crate::domain::types::XyChunk::Done {
                finish_reason: crate::domain::message::XyStopReason::ToolUse,
                usage: None,
            },
        ];
        let mut agent = make_agent_with_tools(
            chunks,
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::runtime_protocol::XyTool>
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
                } if !message_end_seen => {
                    if content.iter().any(|p| {
                        matches!(
                            p,
                            AgentPart::ToolCall {
                                name,
                                ..
                            } if name == "mock_tool"
                        )
                    }) {
                        saw_intent_update = true;
                    }
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
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![
            crate::domain::types::XyChunk::ToolCallEnd {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args: serde_json::json!({}),
            },
            crate::domain::types::XyChunk::Done {
                finish_reason: crate::domain::message::XyStopReason::ToolUse,
                usage: None,
            },
        ];
        let mut agent = make_agent_with_tools(
            chunks,
            ToolSet::from_iter(vec![
                Arc::new(StreamingMockTool) as Arc<dyn crate::runtime_protocol::XyTool>
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
    async fn test_before_hook_denies_tool_call() {
        use crate::agent::runtime::hooks::BeforeToolHook;
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::domain::types::XyChunk::ToolCallEnd {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({"input": "x"}),
        }];
        let mut agent = make_agent_with_tools(
            chunks,
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::runtime_protocol::XyTool>
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
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::domain::types::XyChunk::ToolCallEnd {
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
                Arc::new(MockTool) as Arc<dyn crate::runtime_protocol::XyTool>
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
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::domain::types::XyChunk::ToolCallEnd {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({"input": "x"}),
        }];
        let mut agent = make_agent_with_tools(
            chunks.clone(),
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::runtime_protocol::XyTool>
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
        rounds: std::sync::Mutex<Vec<Vec<crate::domain::types::XyChunk>>>,
    }
    #[async_trait::async_trait]
    impl XyModel for StatefulMockModel {
        fn name(&self) -> &str {
            "stateful-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<AgentMessage>,
            _tools: &[crate::domain::types::XyToolSchema],
            _stream: bool,
            _options: crate::runtime_protocol::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let chunks = self.rounds.lock().unwrap().remove(0);
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }

    fn make_agent_with_rounds(
        rounds: Vec<Vec<crate::domain::types::XyChunk>>,
        tools: ToolSet,
    ) -> AgentRuntime {
        use crate::runtime_protocol::XyModelBuilder;
        let reg = mock_model_registry();
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let builder: XyModelBuilder = Arc::new(move |_| {
            Ok(Arc::new(StatefulMockModel {
                rounds: std::sync::Mutex::new(rounds.clone()),
            }) as Arc<dyn XyModel>)
        });
        let session = AgentCapabilities::new(
            reg,
            tools,
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            50,
            0.8,
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
        AgentRuntime::new(session)
    }

    #[tokio::test]
    async fn tool_call_then_continuation_round_reaches_final_text() {
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        // Round 1: model calls a tool. The provider appends Done after the
        // FunctionCall (openai.rs:156-176), so Done sets `done=true` — this is
        // exactly the case where the old `if done { break }` wrongly aborted.
        // Round 2: model gives the final text reply (no tool call) + Done.
        let done_stop = || crate::domain::types::XyChunk::Done {
            finish_reason: crate::domain::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![
            vec![
                crate::domain::types::XyChunk::ToolCallEnd {
                    id: "call-1".into(),
                    name: "mock_tool".into(),
                    args: serde_json::json!({"input": "x"}),
                },
                done_stop(),
            ],
            vec![
                crate::domain::types::XyChunk::TextDelta("the answer is 42".into()),
                done_stop(),
            ],
        ];
        let mut agent = make_agent_with_rounds(
            rounds,
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::runtime_protocol::XyTool>
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
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let done_stop = || crate::domain::types::XyChunk::Done {
            finish_reason: crate::domain::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![vec![
            crate::domain::types::XyChunk::TextDelta("ok".into()),
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
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let done_stop = || crate::domain::types::XyChunk::Done {
            finish_reason: crate::domain::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![
            vec![
                crate::domain::types::XyChunk::TextDelta("first".into()),
                done_stop(),
            ],
            vec![
                crate::domain::types::XyChunk::TextDelta("second".into()),
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
    async fn abort_before_run_does_not_stick_to_next_run() {
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let done_stop = || crate::domain::types::XyChunk::Done {
            finish_reason: crate::domain::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![vec![
            crate::domain::types::XyChunk::TextDelta("recovered".into()),
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
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let done_stop = || crate::domain::types::XyChunk::Done {
            finish_reason: crate::domain::message::XyStopReason::Stop,
            usage: None,
        };
        // Each `run` rebuilds the mock from the same round template — we only
        // assert the second run is not sticky-aborted (c482).
        let rounds = vec![vec![
            crate::domain::types::XyChunk::TextDelta("ok".into()),
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
        use crate::domain::lifecycle::XyEvent;
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
                _messages: Vec<AgentMessage>,
                _tools: &[crate::domain::types::XyToolSchema],
                _stream: bool,
                _options: crate::runtime_protocol::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                let polled = self.polled.clone();
                Ok(Box::pin(async_stream::stream! {
                    for i in 0..80u32 {
                        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                        polled.fetch_add(1, Ordering::SeqCst);
                        yield Ok(crate::domain::types::XyChunk::TextDelta(format!("c{i}")));
                    }
                    yield Ok(crate::domain::types::XyChunk::Done {
                        finish_reason: crate::domain::message::XyStopReason::Stop,
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
        let builder: crate::runtime_protocol::XyModelBuilder = Arc::new(move |_| {
            Ok(Arc::new(SlowMock {
                polled: polled_for_builder.clone(),
            }) as Arc<dyn XyModel>)
        });
        let session = AgentCapabilities::new(
            reg,
            ToolSet::empty(),
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            50,
            0.8,
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
    }

    #[tokio::test]
    async fn persist_turn_writes_user_and_assistant_messages() {
        use futures::StreamExt;

        let done_stop = || crate::domain::types::XyChunk::Done {
            finish_reason: crate::domain::message::XyStopReason::Stop,
            usage: None,
        };
        let rounds = vec![vec![
            crate::domain::types::XyChunk::TextDelta("hello back".into()),
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

        let done_stop = || crate::domain::types::XyChunk::Done {
            finish_reason: crate::domain::message::XyStopReason::Stop,
            usage: None,
        };

        struct RecordingMockModel {
            seen: std::sync::Arc<std::sync::Mutex<Vec<Vec<AgentMessage>>>>,
            chunks: Vec<crate::domain::types::XyChunk>,
        }

        #[async_trait::async_trait]
        impl XyModel for RecordingMockModel {
            fn name(&self) -> &str {
                "recording-mock"
            }

            async fn generate_stream(
                &self,
                messages: Vec<AgentMessage>,
                _tools: &[crate::domain::types::XyToolSchema],
                _stream: bool,
                _options: crate::runtime_protocol::XyGenerateOptions,
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
                        crate::domain::types::XyChunk::TextDelta("ok".into()),
                        done_stop(),
                    ],
                }) as Arc<dyn XyModel>)
            })
        };
        let mut agent = AgentRuntime::new(AgentCapabilities::new(
            reg,
            ToolSet::empty(),
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            50,
            0.8,
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
            user_texts.iter().any(|t| t == "turn one"),
            "second turn must include first user prompt: {user_texts:?}"
        );
        assert!(user_texts.iter().any(|t| t == "turn two"));
    }

    #[tokio::test]
    async fn dollar_skill_expanded_for_model_history_stays_raw() {
        use crate::domain::resource_types::SkillInfo;
        use crate::domain::source_info::{SourceInfo, SourceOrigin, SourceScope};
        use futures::StreamExt;

        struct RecordingMockModel {
            seen: std::sync::Arc<std::sync::Mutex<Vec<Vec<AgentMessage>>>>,
        }

        #[async_trait::async_trait]
        impl XyModel for RecordingMockModel {
            fn name(&self) -> &str {
                "recording-mock"
            }

            async fn generate_stream(
                &self,
                messages: Vec<AgentMessage>,
                _tools: &[crate::domain::types::XyToolSchema],
                _stream: bool,
                _options: crate::runtime_protocol::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                self.seen.lock().unwrap().push(messages);
                Ok(Box::pin(futures::stream::iter(
                    [
                        crate::domain::types::XyChunk::TextDelta("ok".into()),
                        crate::domain::types::XyChunk::Done {
                            finish_reason: crate::domain::message::XyStopReason::Stop,
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
        let mut agent = AgentRuntime::new(AgentCapabilities::new(
            reg,
            ToolSet::empty(),
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            50,
            0.8,
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
}
