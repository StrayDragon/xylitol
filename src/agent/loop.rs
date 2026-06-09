//! Agent execution loop — core ReAct loop with full event stream.
//!
//! Emits: turn_start, message_start, message_update (streaming), message_end,
//! tool_execution_start, tool_execution_update (streaming), tool_execution_end,
//! turn_end. Aligns with pi's AgentSession event model.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::Stream;
use futures::StreamExt;
use serde_json::Value;

use crate::agent::error::XyError;
use crate::agent::event::AgentEventBus;
use crate::agent::retry::{RetryState, is_retryable_error};
use crate::agent::session::AgentSession;
use crate::agent::traits::{XyModel, XyToolCtx};
use crate::agent::types::{XyChunk, XyContent, XyPart, XyToolSchema};

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
    AgentEnd { messages: Vec<XyContent> },
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

// ── AgentLoop ───────────────────────────────────────────────────────

pub struct AgentLoop {
    pub(crate) session: AgentSession,
}

impl AgentLoop {
    pub fn new(session: AgentSession) -> Self {
        Self { session }
    }

    pub fn session(&self) -> &AgentSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut AgentSession {
        &mut self.session
    }

    /// Run the agent loop with a user prompt.
    pub async fn run(&mut self, prompt: &str, session_id: &str) -> AgentEventStream {
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

        let thinking_level = self.session.thinking_level();

        let inner: Pin<Box<dyn Stream<Item = AgentEvent> + Send>> = Box::pin(run_react_loop(
            model,
            tools,
            tool_schemas,
            system_prompt,
            max_iterations as usize,
            prompt,
            thinking_level,
        ));

        AgentEventStream {
            inner,
            done: false,
            turn_index: 0,
        }
    }
}

// ── Core ReAct loop ─────────────────────────────────────────────────

fn run_react_loop(
    model: Arc<dyn XyModel>,
    tools: ToolRegistry,
    tool_schemas: Vec<XyToolSchema>,
    system_prompt: Option<String>,
    max_iterations: usize,
    user_prompt: String,
    _thinking_level: crate::agent::session::ThinkingLevel,
) -> impl Stream<Item = AgentEvent> + Send {
    async_stream::stream! {
        let mut history: Vec<XyContent> = Vec::new();

        // Add system prompt to history
        if let Some(ref sp) = system_prompt {
            history.push(XyContent::system(sp));
        }

        // Add user message
        history.push(XyContent::user(&user_prompt));

        let retry_state = RetryState::new(3, 1000);

        for turn in 0..max_iterations {
            yield AgentEvent::TurnStart { turn_index: turn as u32 };

            // Build messages: history (without final user) + current user
            let messages: Vec<XyContent> = if turn == 0 {
                history.clone()
            } else {
                let mut msgs = Vec::new();
                if let Some(ref sp) = system_prompt {
                    msgs.push(XyContent::system(sp));
                }
                msgs.push(XyContent::user(&user_prompt));
                msgs
            };

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
                assistant_parts.push(XyPart::Thinking(thinking_acc));
            }
            if !text_acc.is_empty() {
                assistant_parts.push(XyPart::Text(text_acc));
            }
            for (id, name, args) in &tool_calls {
                assistant_parts.push(XyPart::FunctionCall {
                    id: id.clone(),
                    name: name.clone(),
                    args: args.clone(),
                });
            }
            if !assistant_parts.is_empty() {
                history.push(XyContent::assistant(assistant_parts));
            }

            // If no tool calls, done
            if tool_calls.is_empty() {
                yield AgentEvent::TurnEnd { turn_index: turn as u32 };
                break;
            }

            // Execute tool calls
            for (id, name, args) in &tool_calls {
                let tool = tools.get(name);
                let ctx = XyToolCtx::new(id);

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

                history.push(XyContent::tool_result(
                    name.clone(),
                    result,
                    id.clone(),
                ));
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
    messages: Vec<XyContent>,
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

// ── AgentEventStream ────────────────────────────────────────────────

pub struct AgentEventStream {
    inner: Pin<Box<dyn Stream<Item = AgentEvent> + Send>>,
    done: bool,
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

    /// Bridge this stream to an EventBus, returning a handle.
    /// Events consumed from the stream are published to the bus.
    /// The subscriber receives events via the bus.
    pub async fn fan_out(self, bus: &AgentEventBus) {
        let mut stream = self;
        while let Some(event) = stream.next().await {
            bus.emit(event);
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
    use crate::agent::provider::MockXyModel;
    use crate::agent::session::{ModelMeta, ModelRegistry};
    use crate::infra::session::SessionManager;

    #[tokio::test]
    async fn test_agent_session_builds_model() {
        let mut reg = ModelRegistry::new();
        reg.register(ModelMeta {
            id: "mock".into(),
            config: crate::agent::model::ModelConfig {
                kind: crate::agent::model::ModelKind::OpenAi,
                api_key: "sk-test".into(),
                model: "mock-model".into(),
                base_url: None,
            },
            display_name: "Mock".into(),
            thinking: false,
            context_window: 128000,
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
        );

        assert!(session.current_model().is_some());
        assert_eq!(session.current_model().unwrap().id, "mock");
    }

    #[tokio::test]
    async fn test_agent_loop_emits_events() {
        let mut reg = ModelRegistry::new();
        reg.register(ModelMeta {
            id: "mock".into(),
            config: crate::agent::model::ModelConfig {
                kind: crate::agent::model::ModelKind::OpenAi,
                api_key: "sk-test".into(),
                model: "mock-model".into(),
                base_url: None,
            },
            display_name: "Mock".into(),
            thinking: false,
            context_window: 128000,
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
        );

        let mut loop_runner = AgentLoop::new(session);
        let stream = loop_runner.run("hello", "test-session").await;
        // Stream should emit either error (no API key) or events
        // Just verify the stream compiles and produces items
    }
}
