//! Agent execution loop — core ReAct loop powered by XyModel / XyTool.
//!
//! Provides [`AgentLoop`] which owns an `XyModel`, `ToolRegistry`, and `XySession`
//! and emits [`AgentEvent`] items that consumer layers (Print / TUI / ACP) subscribe to.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::Stream;

use crate::agent::repeat::{DetectionConfig, RepeatDetector};
use crate::agent::session::XySession;
use crate::agent::tools::ToolRegistry;
use crate::agent::traits::{XyModel, XyToolCtx};
use crate::agent::types::{XyChunk, XyContent, XyPart, XyToolSchema};
use crate::infra::hooks::{DispatchResult, HookDispatcher, HookEvent, HookPhase};

// ---------------------------------------------------------------------------
// AgentEvent
// ---------------------------------------------------------------------------

/// Events emitted during agent execution.
#[derive(Debug, Clone)]
pub(crate) enum AgentEvent {
    /// Streaming text delta from the LLM.
    TextDelta(String),
    /// Streaming thinking/reasoning delta from a thinking-capable model.
    ThinkingDelta(String),
    /// A tool call was requested by the LLM.
    ToolCallStart {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    /// A tool call completed with a result.
    ToolCallEnd {
        id: String,
        result: serde_json::Value,
    },
    /// A full agent step (LLM round-trip) completed.
    StepComplete { step: u32, summary: String },
    /// An error occurred during execution.
    Error(AgentError),
    /// Repeat detection triggered — model output loop was interrupted.
    RepeatDetected {
        consecutive_hits: u32,
        window_repeat_ratio: f64,
    },
}

// ---------------------------------------------------------------------------
// AgentError
// ---------------------------------------------------------------------------

/// Errors that can occur during agent execution.
#[derive(Debug, Clone, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum AgentError {
    /// Error from the LLM provider (API error, rate limit, etc.).
    #[error("LLM error: {message}")]
    LlmError { message: String, retryable: bool },

    /// Error during tool execution.
    #[error("Tool '{tool}' failed: {message}")]
    ToolError { tool: String, message: String },

    /// Session persistence error.
    #[error("Session error: {0}")]
    SessionError(String),

    /// Configuration error (invalid model, missing key, etc.).
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

// ---------------------------------------------------------------------------
// AgentLoop
// ---------------------------------------------------------------------------

/// High-level agent execution loop.
pub(crate) struct AgentLoop {
    model: Arc<dyn XyModel>,
    tools: ToolRegistry,
    session: Arc<dyn XySession>,
    system_prompt: Option<String>,
    max_iterations: usize,
    app_name: String,
    step_counter: std::sync::atomic::AtomicU32,
    hooks: Option<HookDispatcher>,
}

impl AgentLoop {
    pub(crate) async fn new(
        tool_registry: &ToolRegistry,
        profile: crate::agent::profile::ResolvedProfile,
        session: Arc<dyn XySession>,
        app_name: String,
        hooks_config: Option<&crate::infra::config::types::HooksConfig>,
    ) -> Result<Self, AgentError> {
        let model = profile
            .model_config
            .build()
            .map_err(|e| AgentError::ConfigError(format!("build model: {e}")))?;

        let tools = tool_registry.clone();
        let hooks = hooks_config.map(HookDispatcher::new);

        Ok(Self {
            model,
            tools,
            session,
            system_prompt: profile.system_prompt.clone(),
            max_iterations: profile.max_iterations as usize,
            app_name,
            step_counter: std::sync::atomic::AtomicU32::new(0),
            hooks,
        })
    }

    /// Create an AgentLoop with a pre-built model (for testing).
    #[cfg(test)]
    pub(crate) fn with_model(
        model: Arc<dyn XyModel>,
        tool_registry: &ToolRegistry,
        session: Arc<dyn XySession>,
        system_prompt: Option<String>,
        max_iterations: usize,
    ) -> Self {
        Self {
            model,
            tools: tool_registry.clone(),
            session,
            system_prompt,
            max_iterations,
            app_name: "test".into(),
            step_counter: std::sync::atomic::AtomicU32::new(0),
            hooks: None,
        }
    }

    /// Run the agent with the given prompt and session.
    pub(crate) async fn run(
        &self,
        prompt: &str,
        session_id: &str,
        repeat_detection: Option<DetectionConfig>,
    ) -> Result<AgentEventStream, AgentError> {
        // Load-or-create session
        if !self.session.exists(session_id).await {
            self.session
                .create(session_id)
                .await
                .map_err(|e| AgentError::SessionError(format!("create session: {e}")))?;
        }

        let step = self
            .step_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let detector = repeat_detection.map(RepeatDetector::new);

        let model = self.model.clone();
        let tools = self.tools.clone();
        let session = self.session.clone();
        let system_prompt = self.system_prompt.clone();
        let max_iterations = self.max_iterations;
        let prompt = prompt.to_string();
        let session_id = session_id.to_string();

        let inner = Box::pin(run_react_loop(ReactLoopParams {
            model,
            tools,
            session,
            system_prompt,
            max_iterations,
            prompt,
            session_id,
        }));

        Ok(AgentEventStream {
            inner,
            step,
            done: false,
            pending: std::collections::VecDeque::new(),
            detector,
        })
    }

    pub(crate) async fn dispatch_hook(
        &self,
        event: &HookEvent,
        phase: HookPhase,
    ) -> DispatchResult {
        match &self.hooks {
            Some(d) => d.dispatch(event, phase).await,
            None => DispatchResult::Allowed,
        }
    }

    pub(crate) fn has_hooks(&self) -> bool {
        self.hooks.as_ref().is_some_and(|d| !d.is_empty())
    }

    pub(crate) fn hooks_ref(&self) -> Option<&HookDispatcher> {
        self.hooks.as_ref()
    }
}

/// Parameters for the core ReAct loop.
struct ReactLoopParams {
    model: Arc<dyn XyModel>,
    tools: ToolRegistry,
    session: Arc<dyn XySession>,
    system_prompt: Option<String>,
    max_iterations: usize,
    prompt: String,
    session_id: String,
}

/// The core ReAct loop as an async stream of AgentEvent.
fn run_react_loop(params: ReactLoopParams) -> Pin<Box<dyn Stream<Item = AgentEvent> + Send>> {
    Box::pin(async_stream::stream! {
        use futures::StreamExt;

        let ReactLoopParams {
            model, tools, session, system_prompt, max_iterations, prompt, session_id,
        } = params;

        // Load history + append user message
        let mut history = session.load(&session_id).await.unwrap_or_default();
        history.push(XyContent::user(&prompt));

        // Build tool schemas for the model
        let tool_schemas: Vec<XyToolSchema> = tools
            .list()
            .iter()
            .map(|t| XyToolSchema {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();

        for iteration in 0..max_iterations {
            // Build messages with optional system prompt
            let mut messages = Vec::new();
            if let Some(ref sp) = system_prompt {
                messages.push(XyContent::system(sp));
            }
            messages.extend(history.iter().cloned());

            // Call the model
            let stream_result = model
                .generate_stream(messages, &tool_schemas, true)
                .await;

            let mut chunk_stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    yield AgentEvent::Error(AgentError::LlmError {
                        message: e.to_string(),
                        retryable: true,
                    });
                    return;
                }
            };

            // Accumulate the response
            let mut text_acc = String::new();
            let mut thinking_acc = String::new();
            let mut tool_calls: Vec<(String, String, serde_json::Value)> = Vec::new();
            let mut done = false;

            while let Some(chunk_result) = chunk_stream.next().await {
                match chunk_result {
                    Ok(chunk) => match chunk {
                        XyChunk::TextDelta(text) => {
                            text_acc.push_str(&text);
                            yield AgentEvent::TextDelta(text);
                        }
                        XyChunk::ThinkingDelta(text) => {
                            thinking_acc.push_str(&text);
                            yield AgentEvent::ThinkingDelta(text);
                        }
                        XyChunk::FunctionCall { name, args, id } => {
                            yield AgentEvent::ToolCallStart {
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
                        yield AgentEvent::Error(AgentError::LlmError {
                            message: e.to_string(),
                            retryable: false,
                        });
                        return;
                    }
                }
            }

            // Build assistant message for history
            let mut assistant_parts = Vec::new();
            if !thinking_acc.is_empty() {
                assistant_parts.push(XyPart::Thinking(thinking_acc));
            }
            if !text_acc.is_empty() {
                assistant_parts.push(XyPart::Text(text_acc));
            }
            for (id, name, args) in &tool_calls {
                assistant_parts.push(XyPart::FunctionCall {
                    name: name.clone(),
                    args: args.clone(),
                    id: id.clone(),
                });
            }
            if !assistant_parts.is_empty() {
                history.push(XyContent::assistant(assistant_parts));
            }

            // If no tool calls, we're done
            if tool_calls.is_empty() {
                let _ = session.save(&session_id, &history).await;
                return;
            }

            // Execute tool calls and add results to history
            for (id, name, args) in &tool_calls {
                let tool = tools.get(name);
                let result = match tool {
                    Some(t) => {
                        let ctx = XyToolCtx { call_id: id.clone() };
                        match t.execute(&ctx, args.clone()).await {
                            Ok(output) => output,
                            Err(e) => {
                                let error_msg = format!("Tool error: {e}");
                                yield AgentEvent::Error(AgentError::ToolError {
                                    tool: name.clone(),
                                    message: error_msg.clone(),
                                });
                                error_msg
                            }
                        }
                    }
                    None => {
                        let msg = format!("Unknown tool: {name}");
                        yield AgentEvent::Error(AgentError::ToolError {
                            tool: name.clone(),
                            message: msg.clone(),
                        });
                        msg
                    }
                };

                let result_value: serde_json::Value = serde_json::from_str(&result)
                    .unwrap_or(serde_json::Value::String(result.clone()));

                yield AgentEvent::ToolCallEnd {
                    id: id.clone(),
                    result: result_value,
                };

                history.push(XyContent::tool_result(
                    name.clone(),
                    result,
                    id.clone(),
                ));
            }

            let _ = session.save(&session_id, &history).await;

            if done && iteration + 1 >= max_iterations {
                break;
            }
        }
    })
}

// ---------------------------------------------------------------------------
// AgentEventStream
// ---------------------------------------------------------------------------

/// Stream wrapper that adds repeat detection on top of the inner event stream.
pub(crate) struct AgentEventStream {
    inner: Pin<Box<dyn Stream<Item = AgentEvent> + Send>>,
    step: u32,
    done: bool,
    pending: std::collections::VecDeque<AgentEvent>,
    detector: Option<RepeatDetector>,
}

impl Stream for AgentEventStream {
    type Item = AgentEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        if let Some(next) = self.pending.pop_front() {
            return Poll::Ready(Some(self.apply_repeat_detection(next)));
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => Poll::Ready(Some(self.apply_repeat_detection(event))),
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(Some(AgentEvent::StepComplete {
                    step: self.step,
                    summary: String::new(),
                }))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AgentEventStream {
    fn apply_repeat_detection(&mut self, agent_event: AgentEvent) -> AgentEvent {
        if let AgentEvent::TextDelta(ref text) = agent_event
            && let Some(ref mut detector) = self.detector
            && let Some(result) = detector.feed(text)
        {
            self.pending.clear();
            self.done = true;
            return AgentEvent::RepeatDetected {
                consecutive_hits: result.consecutive_hits,
                window_repeat_ratio: result.window_repeat_ratio,
            };
        }
        agent_event
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::provider::MockXyModel;
    use crate::agent::session::InMemorySession;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_react_loop_text_response() {
        let model = Arc::new(MockXyModel::new("test").with_text("Hello from mock!"));
        let session: Arc<dyn XySession> = Arc::new(InMemorySession::new());
        session.create("s1").await.unwrap();
        let tool_schemas = Vec::new();

        let msgs = vec![XyContent::user("say hi")];
        let stream = model
            .generate_stream(msgs.clone(), &tool_schemas, true)
            .await
            .unwrap();
        let chunks: Vec<_> = stream.collect::<Vec<_>>().await;
        assert!(
            chunks
                .iter()
                .any(|c| matches!(c, Ok(XyChunk::TextDelta(t)) if t.contains("Hello")))
        );
    }

    #[tokio::test]
    async fn test_agent_loop_emits_step_complete() {
        let model = Arc::new(MockXyModel::new("test").with_text("done"));
        let session: Arc<dyn XySession> = Arc::new(InMemorySession::new());
        let tools = ToolRegistry::new();

        let inner = run_react_loop(ReactLoopParams {
            model,
            tools,
            session: session.clone(),
            system_prompt: None,
            max_iterations: 5,
            prompt: "hi".into(),
            session_id: "s1".into(),
        });
        let mut event_stream = AgentEventStream {
            inner: Box::pin(inner),
            step: 1,
            done: false,
            pending: std::collections::VecDeque::new(),
            detector: None,
        };

        let mut events = Vec::new();
        while let Some(e) = event_stream.next().await {
            events.push(e);
        }

        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentEvent::TextDelta(t) if t.contains("done")))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentEvent::StepComplete { .. }))
        );
    }

    #[tokio::test]
    async fn test_session_preserves_across_runs() {
        let model = Arc::new(MockXyModel::new("test").with_text("ok"));
        let session: Arc<dyn XySession> = Arc::new(InMemorySession::new());
        let tools = ToolRegistry::new();

        // First run
        let inner = run_react_loop(ReactLoopParams {
            model: model.clone(),
            tools: tools.clone(),
            session: session.clone(),
            system_prompt: None,
            max_iterations: 5,
            prompt: "first".into(),
            session_id: "s1".into(),
        });
        let mut s = AgentEventStream {
            inner: Box::pin(inner),
            step: 1,
            done: false,
            pending: std::collections::VecDeque::new(),
            detector: None,
        };
        while s.next().await.is_some() {}

        let h1 = session.load("s1").await.unwrap();
        assert!(!h1.is_empty());

        // Second run on same session
        let inner = run_react_loop(ReactLoopParams {
            model: model.clone(),
            tools: tools.clone(),
            session: session.clone(),
            system_prompt: None,
            max_iterations: 5,
            prompt: "second".into(),
            session_id: "s1".into(),
        });
        let mut s = AgentEventStream {
            inner: Box::pin(inner),
            step: 2,
            done: false,
            pending: std::collections::VecDeque::new(),
            detector: None,
        };
        while s.next().await.is_some() {}

        let h2 = session.load("s1").await.unwrap();
        assert!(h2.len() > h1.len(), "history should grow across runs");
    }
}
