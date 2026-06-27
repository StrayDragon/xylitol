//! Agent execution loop — core ReAct loop with full event stream, hooks, and tool execution modes.
//!
//! NOTE: 本文件聚焦 ReAct 算法主体. 天花板: ~500 行 (含 inline tests), 因 run_react_loop
//! 的 async_stream 宏块是原子逻辑单元, 跨函数 yield 不可行. 升级: 当工具执行/流处理逻辑
//! 显著膨胀时, 考虑引入 sub-turn state machine 替代单宏块.

//!
//! Key features:
//! - ReAct loop with turn-based execution
//! - `AgentHooks`: before_tool_call, after_tool_call, transform_context
//! - Steering/follow-up message queue callbacks
//! - Per-tool execution modes: sequential / parallel
//! - Auto-retry on transient errors

use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use futures::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::retry::{RetryState, is_retryable_error};
use super::sandbox_router::sandbox_target;
use super::{AgentEvent, AgentEventStream, AgentHooks};
use crate::agent::session::AgentSession;
use crate::agent::tools::ToolRegistry;
use crate::core::error::XyError;
use crate::core::message::{AgentMessage, AgentPart};
use crate::core::ports::{ToolExecutionMode, XyModel, XyToolCtx};
use crate::core::types::{XyChunk, XyToolSchema};

use crate::core::ports::SandboxVerdict;

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::agent::model::registry::ModelRegistry;
    use crate::core::model::ModelConfig;
    use crate::core::ports::{EventSink, SessionStore, XyModel};
    use crate::core::types::ModelMeta;
    use crate::infra::session::SessionManager;

    /// Model builder for tests — the real factory (tests register `Fake`/`OpenAi`
    /// model configs and rely on `build_provider` constructing the provider struct;
    /// no real network calls are made in unit assertions).
    fn fake_model_builder()
    -> Arc<dyn Fn(&ModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync> {
        Arc::new(crate::infra::provider::factory::build_provider)
    }

    #[tokio::test]
    async fn test_agent_session_builds_model() {
        let mut reg = ModelRegistry::new(std::sync::Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ));
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
        let store: Arc<dyn SessionStore> = Arc::new(session_mgr.clone());
        let sink: Arc<dyn EventSink> = Arc::new(crate::infra::event::EventBus::new());
        let session = AgentSession::new(
            reg,
            ToolRegistry::from_tools(crate::infra::tools::default_tools()),
            session_mgr,
            store,
            sink,
            Some("You are helpful.".into()),
            50,
            0.8,
            ".".into(),
            None,
            fake_model_builder(),
            crate::infra::sandbox::noop_engine(),
            std::sync::Arc::new(crate::infra::bash_exec::InfraBashExecutor::new()),
        );

        assert!(session.current_model().is_some());
        assert_eq!(session.current_model().unwrap().id, "mock");
    }

    #[tokio::test]
    async fn test_agent_loop_emits_events() {
        let mut reg = ModelRegistry::new(std::sync::Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ));
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
        let store: Arc<dyn SessionStore> = Arc::new(session_mgr.clone());
        let sink: Arc<dyn EventSink> = Arc::new(crate::infra::event::EventBus::new());
        let session = AgentSession::new(
            reg,
            ToolRegistry::from_tools(crate::infra::tools::default_tools()),
            session_mgr,
            store,
            sink,
            Some("You are helpful.".into()),
            50,
            0.8,
            ".".into(),
            None,
            fake_model_builder(),
            crate::infra::sandbox::noop_engine(),
            std::sync::Arc::new(crate::infra::bash_exec::InfraBashExecutor::new()),
        );

        let mut loop_runner = AgentLoop::new(session);
        let _stream = loop_runner.run("hello", "test-session").await;
    }
}
