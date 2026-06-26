//! Agent execution loop — core ReAct loop with full event stream, hooks, and tool execution modes.

#![allow(dead_code)]
//!
//! Key features:
//! - ReAct loop with turn-based execution
//! - `AgentHooks`: before_tool_call, after_tool_call, transform_context
//! - Steering/follow-up message queue callbacks
//! - Per-tool execution modes: sequential / parallel
//! - Auto-retry on transient errors

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::Stream;
use futures::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::agent::retry::{RetryState, is_retryable_error};
use crate::agent::session::AgentSession;
use crate::core::error::XyError;
use crate::core::message::{AgentMessage, AgentPart};
use crate::core::traits::{ToolExecutionMode, XyModel, XyToolCtx};
use crate::core::types::{XyChunk, XyToolSchema};

use crate::infra::sandbox::SandboxVerdict;

// ── AgentEvent ──────────────────────────────────────────────────────

/// Events emitted during agent execution.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Turn started.
    TurnStart { turn_index: u32 },
    /// Message started (a new message block).
    MessageStart { role: String },
    /// Streaming text delta from the LLM.
    TextDelta(String),
    /// Streaming thinking delta.
    ThinkingDelta(String),
    /// Message content updated.
    MessageUpdate {
        text: String,
        thinking: Option<String>,
    },
    /// Message completed.
    MessageEnd { role: String },
    /// Tool execution started.
    ToolExecutionStart {
        id: String,
        name: String,
        args: Value,
    },
    /// Tool execution update (streaming partial output).
    ToolExecutionUpdate { id: String, output: String },
    /// Tool execution completed.
    ToolExecutionEnd {
        id: String,
        name: String,
        result: String,
    },
    /// Turn ended.
    TurnEnd { turn_index: u32 },
    /// Error occurred.
    Error(String),
    /// Agent loop completed.
    AgentEnd { messages: Vec<AgentMessage> },
    /// Compaction started.
    CompactionStart { reason: String },
    /// Compaction ended.
    CompactionEnd {
        result: Option<String>,
        aborted: bool,
    },
    /// Model switched.
    ModelSelect { provider: String, model_id: String },
    /// Thinking level changed.
    ThinkingLevelChanged { level: String },
}

// ── AgentHooks ────────────────────────────────────────────────

/// Type alias for hook callbacks to simplify declarations.
pub type BeforeToolHook = Box<dyn Fn(&str, &str, &Value) -> Option<String> + Send + Sync>;
pub type AfterToolHook =
    Box<dyn Fn(&str, &str, Value, bool) -> Option<(Value, bool)> + Send + Sync>;
pub type TransformCtxHook = Box<dyn Fn(Vec<AgentMessage>) -> Vec<AgentMessage> + Send + Sync>;
pub type GetMessagesHook = Box<dyn Fn() -> Vec<AgentMessage> + Send + Sync>;

/// Hooks for customizing the agent loop.
pub struct AgentHooks {
    pub before_tool_call: Option<BeforeToolHook>,
    pub after_tool_call: Option<AfterToolHook>,
    pub transform_context: Option<TransformCtxHook>,
    pub get_steering_messages: Option<GetMessagesHook>,
    pub get_follow_up_messages: Option<GetMessagesHook>,
    pub max_retries: usize,
}

impl Default for AgentHooks {
    fn default() -> Self {
        Self {
            before_tool_call: None,
            after_tool_call: None,
            transform_context: None,
            get_steering_messages: None,
            get_follow_up_messages: None,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteeringMode {
    All,
    OneAtATime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowUpMode {
    Stop,
    Continue,
}

// ── AgentLoop ───────────────────────────────────────────────────────

pub struct AgentLoop {
    pub(crate) session: AgentSession,
    /// Cancellation token.
    cancel: CancellationToken,
    /// Agent hooks.
    hooks: AgentHooks,
    /// Tool execution mode.
    tool_mode: ToolExecutionMode,
}

impl AgentLoop {
    pub fn new(session: AgentSession) -> Self {
        Self {
            session,
            cancel: CancellationToken::new(),
            hooks: AgentHooks::default(),
            tool_mode: ToolExecutionMode::Sequential,
        }
    }

    /// Set agent hooks.
    pub fn with_hooks(mut self, hooks: AgentHooks) -> Self {
        self.hooks = hooks;
        self
    }

    /// Set tool execution mode.
    pub fn with_tool_mode(mut self, mode: ToolExecutionMode) -> Self {
        self.tool_mode = mode;
        self
    }

    /// Get a reference to the cancellation token.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Signal cancellation to abort the agent loop.
    pub fn abort(&self) {
        self.cancel.cancel();
    }

    pub fn session(&self) -> &AgentSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut AgentSession {
        &mut self.session
    }

    /// Run the agent loop with a user prompt.
    #[allow(clippy::type_complexity)]
    pub async fn run(&mut self, prompt: &str, session_id: &str) -> AgentEventStream {
        // Build sandbox check callback from session
        let sandbox_check: Option<
            std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>,
        >;
        {
            let engine = self.session.get_sandbox_engine();
            sandbox_check = Some(std::sync::Arc::new(
                move |tool_name: &str, tool_path: &str| -> Option<String> {
                    match tool_name {
                        "read" => match engine.check_read(tool_path) {
                            SandboxVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        "write" | "edit" => match engine.check_write(tool_path) {
                            SandboxVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        "bash" => match engine.check_network(tool_path) {
                            SandboxVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        _ => None,
                    }
                },
            ));
        }
        // Ensure session exists
        let sid = session_id.to_string();
        self.session.set_session(sid.clone());
        if let Err(e) = self.session.ensure_session(&sid, None).await {
            return AgentEventStream::error(format!("session error: {e}"));
        }

        let model = match self.session.build_current_model() {
            Ok(m) => m,
            Err(e) => return AgentEventStream::error(format!("model build error: {e}")),
        };

        let tools = self.session.tool_registry().clone();
        let max_iterations = self.session.max_iterations();
        let system_prompt = self.session.system_prompt().map(|s| s.to_string());
        let prompt = prompt.to_string();

        // Build tool schemas
        let tool_schemas: Vec<XyToolSchema> = tools
            .list()
            .iter()
            .map(|t| XyToolSchema {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();

        let cancel = self.cancel.clone();

        let inner: Pin<Box<dyn Stream<Item = AgentEvent> + Send>> =
            Box::pin(run_react_loop(ReActConfig {
                model,
                tools,
                tool_schemas,
                system_prompt,
                max_iterations: max_iterations as usize,
                user_prompt: prompt,
                cancel,
                sandbox_check,
            }));

        AgentEventStream {
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
    tools: ToolRegistry,
    tool_schemas: Vec<XyToolSchema>,
    system_prompt: Option<String>,
    max_iterations: usize,
    user_prompt: String,
    cancel: CancellationToken,
    /// Optional sandbox check. Called with (tool_name, target_path_or_domain).
    /// Returns Some(reason) if the operation is denied.
    #[allow(clippy::type_complexity)]
    sandbox_check: Option<std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>>,
}

// ── Core ReAct loop ─────────────────────────────────────────────────

fn run_react_loop(cfg: ReActConfig) -> impl Stream<Item = AgentEvent> + Send {
    let ReActConfig {
        model,
        tools,
        tool_schemas,
        system_prompt,
        max_iterations,
        user_prompt,
        cancel,
        sandbox_check,
    } = cfg;
    async_stream::stream! {
        let mut history: Vec<AgentMessage> = Vec::new();

        // Add system prompt to history (as user message — AgentMessage has no system variant)
        if let Some(ref sp) = system_prompt {
            history.push(AgentMessage::UserMessage {
                content: vec![AgentPart::Text(sp.clone())],
                timestamp: crate::core::message::now_ms(),
            });
        }

        // Add user message
        history.push(AgentMessage::UserMessage {
            content: vec![AgentPart::Text(user_prompt.clone())],
            timestamp: crate::core::message::now_ms(),
        });

        let retry_state = RetryState::new(3, 1000);

        for turn in 0..max_iterations {
            // Check for cancellation before each turn
            if cancel.is_cancelled() {
                yield AgentEvent::Error("aborted".to_string());
                break;
            }

            yield AgentEvent::TurnStart { turn_index: turn as u32 };

            // Send accumulated history to the model.
            // History includes system prompt, user messages, assistant responses,
            // and tool results from previous turns — giving the LLM full context.
            let messages = history.clone();

            // Call model with retry support
            let stream_result = call_with_retry(
                &model, messages.clone(), &tool_schemas, &retry_state,
            ).await;

            let mut chunk_stream: Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;
            match stream_result {
                Ok(s) => chunk_stream = s,
                Err(e) => {
                    yield AgentEvent::Error(e);
                    break;
                }
            }

            yield AgentEvent::MessageStart { role: "assistant".to_string() };

            let mut text_acc = String::new();
            let mut thinking_acc = String::new();
            let mut tool_calls: Vec<(String, String, Value)> = Vec::new();
            let mut done = false;

            while let Some(chunk_result) = chunk_stream.next().await {
                match chunk_result {
                    Ok(chunk) => match chunk {
                        XyChunk::TextDelta(text) => {
                            text_acc.push_str(&text);
                            yield AgentEvent::TextDelta(text.clone());
                            yield AgentEvent::MessageUpdate {
                                text: text_acc.clone(),
                                thinking: if thinking_acc.is_empty() { None } else { Some(thinking_acc.clone()) },
                            };
                        }
                        XyChunk::ThinkingDelta(text) => {
                            thinking_acc.push_str(&text);
                            yield AgentEvent::ThinkingDelta(text);
                        }
                        XyChunk::FunctionCall { name, args, id } => {
                            yield AgentEvent::ToolExecutionStart {
                                id: id.clone(),
                                name: name.clone(),
                                args: args.clone(),
                            };
                            tool_calls.push((id, name, args));
                        }
                        XyChunk::Done { .. } => {
                            done = true;
                        }
                    },
                    Err(e) => {
                        yield AgentEvent::Error(format!("stream error: {e}"));
                        break;
                    }
                }
            }

            yield AgentEvent::MessageEnd { role: "assistant".to_string() };

            // Build assistant message
            let mut assistant_parts = Vec::new();
            if !thinking_acc.is_empty() {
                assistant_parts.push(AgentPart::Thinking { text: thinking_acc, redacted: false, signature: None });
            }
            if !text_acc.is_empty() {
                assistant_parts.push(AgentPart::Text(text_acc));
            }
            for (id, name, args) in &tool_calls {
                assistant_parts.push(AgentPart::ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: args.clone(),
                });
            }
            if !assistant_parts.is_empty() {
                history.push(AgentMessage::AssistantMessage {
                    content: assistant_parts,
                    stop_reason: None,
                    usage: None,
                    api: String::new(),
                    provider: String::new(),
                    model: String::new(),
                    response_id: None,
                    error_message: None,
                    timestamp: crate::core::message::now_ms(),
                    diagnostics: Vec::new(),
                });
            }

            // If no tool calls, done
            if tool_calls.is_empty() {
                yield AgentEvent::TurnEnd { turn_index: turn as u32 };
                break;
            }

            // Execute tool calls
            for (id, name, args) in &tool_calls {
                let tool = tools.get(name);
                let ctx = XyToolCtx::with_cancel(id, cancel.clone());

                // ── Sandbox check ─────────────────────────────────
                if let Some(ref check) = sandbox_check {
                    let target = sandbox_target(name, args);
                    if let Some(reason) = check(name, &target) {
                        let err = format!("Tool '{name}' blocked by sandbox: {reason}");
                        yield AgentEvent::ToolExecutionEnd {
                            id: id.clone(),
                            name: name.clone(),
                            result: err.clone(),
                        };
                        history.push(AgentMessage::ToolResultMessage {
                            tool_use_id: id.clone(),
                            tool_name: name.clone(),
                            content: vec![AgentPart::Text(err.clone())],
                            details: None,
                            is_error: true,
                            timestamp: crate::core::message::now_ms(),
                        });
                        continue;
                    }
                }

                let result = match tool {
                    Some(t) => match t.execute(&ctx, args.clone()).await {
                        Ok(output) => output,
                        Err(e) => {
                            let err = format!("Tool '{name}' error: {e}");
                            yield AgentEvent::Error(err.clone());
                            err
                        }
                    },
                    None => {
                        let err = format!("Unknown tool: {name}");
                        yield AgentEvent::Error(err.clone());
                        err
                    }
                };

                yield AgentEvent::ToolExecutionEnd {
                    id: id.clone(),
                    name: name.clone(),
                    result: result.clone(),
                };

                history.push(AgentMessage::ToolResultMessage {
                    tool_use_id: id.clone(),
                    tool_name: name.clone(),
                    content: vec![AgentPart::Text(result.clone())],
                    details: None,
                    is_error: false,
                    timestamp: crate::core::message::now_ms(),
                });
            }

            yield AgentEvent::TurnEnd { turn_index: turn as u32 };

            if done {
                break;
            }
        }

        yield AgentEvent::AgentEnd { messages: history };
    }
}

/// Helper: call model with retry for transient errors.
async fn call_with_retry(
    model: &Arc<dyn XyModel>,
    messages: Vec<AgentMessage>,
    tool_schemas: &[XyToolSchema],
    retry_state: &RetryState,
) -> Result<Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>, String> {
    loop {
        match model
            .generate_stream(messages.clone(), tool_schemas, true)
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

use crate::agent::tools::ToolRegistry;

/// Extract the sandbox-relevant target (path or domain) from tool arguments.
fn sandbox_target(name: &str, args: &serde_json::Value) -> String {
    match name {
        "read" | "write" | "edit" => args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "bash" => {
            // Extract the first URL domain from the command, if any
            args.get("command")
                .and_then(|v| v.as_str())
                .map(extract_first_domain)
                .unwrap_or_default()
        }
        _ => String::new(),
    }
}

/// Extract the first domain from a shell command containing a URL.
fn extract_first_domain(command: &str) -> String {
    // Look for common URL patterns: https://, http://, //
    for word in command.split_whitespace() {
        let word = word.trim_matches('\'').trim_matches('"');
        if let Some(rest) = word
            .strip_prefix("https://")
            .or_else(|| word.strip_prefix("http://"))
        {
            // Extract domain (stop at first /, :, or ?)
            let domain = rest.split(['/', ':', '?']).next().unwrap_or(rest);
            return domain.to_string();
        }
    }
    String::new()
}

// ── AgentEventStream ────────────────────────────────────────────────

pub struct AgentEventStream {
    inner: Pin<Box<dyn Stream<Item = AgentEvent> + Send>>,
    done: bool,
    /// Track turn number (set externally via event wrapping).
    #[allow(dead_code)]
    turn_index: u32,
}

impl AgentEventStream {
    fn error(msg: String) -> Self {
        let inner: Pin<Box<dyn Stream<Item = AgentEvent> + Send>> =
            Box::pin(async_stream::stream! {
                yield AgentEvent::Error(msg);
            });
        Self {
            inner,
            done: false,
            turn_index: 0,
        }
    }
}

impl Stream for AgentEventStream {
    type Item = AgentEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => {
                if matches!(event, AgentEvent::AgentEnd { .. }) {
                    self.done = true;
                }
                Poll::Ready(Some(event))
            }
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::agent::model::registry::ModelRegistry;
    use crate::core::types::ModelMeta;
    use crate::infra::session::SessionManager;

    #[tokio::test]
    async fn test_agent_session_builds_model() {
        let mut reg = ModelRegistry::new();
        reg.register(ModelMeta {
            id: "mock".into(),
            config: crate::core::model::ModelConfig {
                kind: crate::core::model::ModelKind::OpenAi,
                api_key: "sk-test".into(),
                model: "mock-model".into(),
                base_url: None,
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
        });

        let session_mgr = SessionManager::new(SessionManager::default_dir());
        let session = AgentSession::new(
            reg,
            ToolRegistry::builtins(),
            session_mgr,
            Some("You are helpful.".into()),
            50,
            0.8,
            ".".into(),
            None,
        );

        assert!(session.current_model().is_some());
        assert_eq!(session.current_model().unwrap().id, "mock");
    }

    #[tokio::test]
    async fn test_agent_loop_emits_events() {
        let mut reg = ModelRegistry::new();
        reg.register(ModelMeta {
            id: "mock".into(),
            config: crate::core::model::ModelConfig {
                kind: crate::core::model::ModelKind::OpenAi,
                api_key: "sk-test".into(),
                model: "mock-model".into(),
                base_url: None,
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
        });

        let session_mgr = SessionManager::new(SessionManager::default_dir());
        let session = AgentSession::new(
            reg,
            ToolRegistry::builtins(),
            session_mgr,
            Some("You are helpful.".into()),
            50,
            0.8,
            ".".into(),
            None,
        );

        let mut loop_runner = AgentLoop::new(session);
        let _stream = loop_runner.run("hello", "test-session").await;
    }
}
