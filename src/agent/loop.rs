//! Agent execution loop — core ReAct loop powered by adk-rust.
//!
//! Provides [`AgentLoop`] which wraps `adk_runner::Runner` / `adk_agent::LlmAgent`
//! and emits [`AgentEvent`] items that consumer layers (Print / TUI / ACP) subscribe to.

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use adk_agent::LlmAgentBuilder;
use adk_core::{Content, Event, Part};
use adk_runner::{Runner, RunnerConfig};
use adk_session::SessionService;
use futures::Stream;

use crate::agent::repeat::{DetectionConfig, RepeatDetector};
use crate::agent::tools::ToolRegistry;

// ---------------------------------------------------------------------------
// AgentEvent
// ---------------------------------------------------------------------------

/// Events emitted during agent execution.
#[derive(Debug, Clone)]
pub(crate) enum AgentEvent {
    /// Streaming text delta from the LLM.
    TextDelta(String),
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
    LlmError {
        message: String,
        /// Whether the operation can be retried.
        retryable: bool,
    },

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

impl From<adk_core::AdkError> for AgentError {
    fn from(e: adk_core::AdkError) -> Self {
        AgentError::LlmError {
            message: e.to_string(),
            retryable: false,
        }
    }
}

// ---------------------------------------------------------------------------
// AgentLoop
// ---------------------------------------------------------------------------

/// High-level agent execution loop.
///
/// Wraps an `adk_runner::Runner` (which itself owns an `LlmAgent`) and
/// provides a `run()` method that streams [`AgentEvent`] items.
pub(crate) struct AgentLoop {
    runner: Runner,
    app_name: String,
    session_service: Arc<dyn SessionService>,
    step_counter: std::sync::atomic::AtomicU32,
}

impl AgentLoop {
    /// Create a new agent loop.
    ///
    /// Builds an `LlmAgent` from the resolved profile (model, prompt, tools, iterations),
    /// then wraps it in a `Runner` for session and lifecycle management.
    pub(crate) async fn new(
        tool_registry: &ToolRegistry,
        profile: crate::agent::profile::ResolvedProfile,
        session_service: Arc<dyn SessionService>,
        app_name: String,
    ) -> Result<Self, AgentError> {
        let model = profile
            .model_config
            .build()
            .map_err(|e| AgentError::ConfigError(format!("build model: {e}")))?;

        let mut builder = LlmAgentBuilder::new("xylitol")
            .model(model)
            .description("xylitol — LLM-Augmented Development Toolkit")
            .max_iterations(profile.max_iterations);

        if let Some(ref prompt) = profile.system_prompt {
            builder = builder.instruction(prompt);
        }

        let tools = tool_registry.filtered(profile.allowed_tools.as_deref());
        for tool in tools {
            builder = builder.tool(tool);
        }

        let agent = builder
            .build()
            .map_err(|e| AgentError::ConfigError(format!("build agent: {e}")))?;

        let runner = Runner::new(RunnerConfig {
            app_name: app_name.clone(),
            agent: Arc::new(agent),
            session_service: session_service.clone(),
            memory_service: None,
            run_config: None,
            compaction_config: None,
            context_cache_config: None,
            cache_capable: None,
            request_context: None,
            cancellation_token: None,
            intra_compaction_config: None,
            intra_compaction_summarizer: None,
        })
        .map_err(|e| AgentError::ConfigError(format!("build runner: {e}")))?;

        Ok(Self {
            runner,
            app_name,
            session_service,
            step_counter: std::sync::atomic::AtomicU32::new(0),
        })
    }

    /// Run the agent with the given prompt and session.
    ///
    /// Auto-creates the session if it does not yet exist (load-or-create
    /// semantics), then runs the agent and returns a stream of [`AgentEvent`]
    /// items.
    ///
    /// Optionally pass repeat detection config to enable loop detection.
    pub(crate) async fn run(
        &self,
        prompt: &str,
        session_id: &str,
        repeat_detection: Option<DetectionConfig>,
    ) -> Result<AgentEventStream, AgentError> {
        // Load-or-create session so callers don't need to manage session lifecycle.
        self.ensure_session(session_id).await?;

        let content = Content::new("user").with_text(prompt);

        let stream = self
            .runner
            .run_str("default-user", session_id, content)
            .await
            .map_err(|e| AgentError::LlmError {
                message: e.to_string(),
                retryable: true,
            })?;

        let step_counter = self.step_counter.load(std::sync::atomic::Ordering::Relaxed) + 1;
        self.step_counter
            .store(step_counter, std::sync::atomic::Ordering::Relaxed);

        let detector = repeat_detection.map(RepeatDetector::new);

        Ok(AgentEventStream {
            inner: stream,
            step: step_counter,
            done: false,
            detector,
        })
    }

    /// Ensure a session exists for the given ID by creating one if absent.
    async fn ensure_session(&self, session_id: &str) -> Result<(), AgentError> {
        use adk_session::CreateRequest;
        use std::collections::HashMap;

        self.session_service
            .create(CreateRequest {
                app_name: self.app_name.clone(),
                user_id: "default-user".into(),
                session_id: Some(session_id.into()),
                state: HashMap::new(),
            })
            .await
            .map(|_| ())
            .or({
                // Session already exists — that's fine.
                Ok(())
            })
    }
}

// ---------------------------------------------------------------------------
// AgentEventStream
// ---------------------------------------------------------------------------

/// Stream that maps `adk_core::Event` items to [`AgentEvent`] items.
pub(crate) struct AgentEventStream {
    inner: Pin<Box<dyn Stream<Item = Result<Event, adk_core::AdkError>> + Send>>,
    step: u32,
    done: bool,
    /// Optional repeat detector. When `Some`, text content is monitored for
    /// repetition loops. On detection, the stream yields `RepeatDetected` and
    /// terminates.
    detector: Option<RepeatDetector>,
}

impl Stream for AgentEventStream {
    type Item = AgentEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                let agent_event = map_adk_event(event, self.step);

                // Feed text deltas through the repeat detector, if active.
                if let AgentEvent::TextDelta(ref text) = agent_event
                    && let Some(ref mut detector) = self.detector
                    && let Some(result) = detector.feed(text)
                {
                    self.done = true;
                    return Poll::Ready(Some(AgentEvent::RepeatDetected {
                        consecutive_hits: result.consecutive_hits,
                        window_repeat_ratio: result.window_repeat_ratio,
                    }));
                }

                Poll::Ready(Some(agent_event))
            }
            Poll::Ready(Some(Err(e))) => {
                self.done = true;
                Poll::Ready(Some(AgentEvent::Error(AgentError::LlmError {
                    message: e.to_string(),
                    retryable: false,
                })))
            }
            Poll::Ready(None) => {
                self.done = true;
                // Yields StepComplete after all events are consumed.
                Poll::Ready(Some(AgentEvent::StepComplete {
                    step: self.step,
                    summary: String::new(),
                }))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// Event mapping
// ---------------------------------------------------------------------------

/// Map an `adk_core::Event` to an [`AgentEvent`].
///
/// Priority: FunctionCall > FunctionResponse > text content.
fn map_adk_event(event: Event, step: u32) -> AgentEvent {
    let content = match event.llm_response.content {
        Some(ref c) => c,
        None => {
            return AgentEvent::TextDelta(String::new());
        }
    };

    // Check for function calls first.
    for part in &content.parts {
        if let Part::FunctionCall { name, args, id, .. } = part {
            let call_id = id.clone().unwrap_or_else(|| format!("{step}-{name}"));
            return AgentEvent::ToolCallStart {
                id: call_id,
                name: name.clone(),
                args: args.clone(),
            };
        }
    }

    // Check for function responses.
    for part in &content.parts {
        if let Part::FunctionResponse {
            function_response,
            id,
        } = part
        {
            let call_id = id
                .clone()
                .unwrap_or_else(|| format!("{step}-{}", function_response.name));
            return AgentEvent::ToolCallEnd {
                id: call_id,
                result: function_response.response.clone(),
            };
        }
    }

    // Extract text content (handles both plain text and thinking+text).
    let text: String = content
        .parts
        .iter()
        .filter_map(|p| {
            if let Part::Text { text } = p {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();

    AgentEvent::TextDelta(text)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use adk_core::LlmResponse;
    use adk_model::MockLlm;
    use adk_session::InMemorySessionService;
    use futures::StreamExt;

    // ── Unit tests for event mapping ─────────────────────────────

    #[test]
    fn test_map_text_event() {
        let mut event = Event::new("inv-1");
        event.llm_response.content = Some(Content::new("assistant").with_text("Hello world"));
        let agent_event = map_adk_event(event, 1);
        assert!(matches!(agent_event, AgentEvent::TextDelta(t) if t == "Hello world"));
    }

    #[test]
    fn test_map_function_call_event() {
        let mut event = Event::new("inv-1");
        event.llm_response.content = Some(Content {
            role: "assistant".into(),
            parts: vec![Part::FunctionCall {
                name: "read".into(),
                args: serde_json::json!({"file_path": "/tmp/test.txt"}),
                id: Some("call-1".into()),
                thought_signature: None,
            }],
        });
        let agent_event = map_adk_event(event, 1);
        match agent_event {
            AgentEvent::ToolCallStart { id, name, .. } => {
                assert_eq!(id, "call-1");
                assert_eq!(name, "read");
            }
            other => panic!("expected ToolCallStart, got {other:?}"),
        }
    }

    #[test]
    fn test_map_function_response_event() {
        let mut event = Event::new("inv-1");
        event.llm_response.content = Some(Content {
            role: "function".into(),
            parts: vec![Part::FunctionResponse {
                function_response: adk_core::FunctionResponseData::new(
                    "read",
                    serde_json::json!({"content": "file content"}),
                ),
                id: Some("call-1".into()),
            }],
        });
        let agent_event = map_adk_event(event, 1);
        match agent_event {
            AgentEvent::ToolCallEnd { id, result } => {
                assert_eq!(id, "call-1");
                assert_eq!(result["content"], "file content");
            }
            other => panic!("expected ToolCallEnd, got {other:?}"),
        }
    }

    // ── Integration tests ────────────────────────────────────────

    /// Build a test agent with MockLlm and no tools.
    fn build_test_agent(mock: MockLlm) -> Result<adk_agent::LlmAgent, adk_core::AdkError> {
        let mut builder = adk_agent::LlmAgentBuilder::new("test-agent")
            .model(Arc::new(mock))
            .description("test agent for integration tests")
            .max_iterations(5);
        builder = builder.tool(Arc::new(crate::agent::tools::read::ReadTool));
        builder.build()
    }

    /// Build a Runner wired to MockLlm + InMemorySessionService.
    async fn build_test_runner(
        agent: adk_agent::LlmAgent,
        session_service: Arc<dyn SessionService>,
        session_id: &str,
    ) -> Result<Runner, adk_core::AdkError> {
        // Pre-create the session so Runner's `get()` succeeds
        session_service
            .create(adk_session::CreateRequest {
                app_name: "xylitol-test".into(),
                user_id: "default-user".into(),
                session_id: Some(session_id.into()),
                state: std::collections::HashMap::new(),
            })
            .await
            .map_err(|e| adk_core::AdkError::session(format!("create session: {e}")))?;

        Runner::new(RunnerConfig {
            app_name: "xylitol-test".into(),
            agent: Arc::new(agent),
            session_service,
            memory_service: None,
            run_config: None,
            compaction_config: None,
            context_cache_config: None,
            cache_capable: None,
            request_context: None,
            cancellation_token: None,
            intra_compaction_config: None,
            intra_compaction_summarizer: None,
        })
    }

    #[tokio::test]
    async fn test_agent_loop_text_response() {
        let mock = MockLlm::new("test-llm").with_response(LlmResponse::new(
            Content::new("assistant").with_text("Hello from mock!"),
        ));
        let agent = build_test_agent(mock).unwrap();
        let session_service = Arc::new(InMemorySessionService::new());
        let runner = build_test_runner(agent, session_service, "test-session-1")
            .await
            .unwrap();

        let content = Content::new("user").with_text("Say hello");
        let mut stream = runner
            .run_str("default-user", "test-session-1", content)
            .await
            .unwrap();

        let mut events: Vec<Event> = Vec::new();
        while let Some(result) = stream.next().await {
            events.push(result.unwrap());
        }

        // Should have at least one event with text content
        assert!(!events.is_empty(), "expected at least one event");
        let has_text = events.iter().any(|e| {
            e.llm_response.content.as_ref().is_some_and(|c| {
                c.parts
                    .iter()
                    .any(|p| matches!(p, Part::Text { text } if text.contains("Hello from mock")))
            })
        });
        assert!(has_text, "expected text response in events");
    }

    #[tokio::test]
    async fn test_agent_loop_event_stream() {
        let mock = MockLlm::new("test-llm").with_response(LlmResponse::new(
            Content::new("assistant").with_text("Hello world"),
        ));
        let agent = build_test_agent(mock).unwrap();
        let session_service = Arc::new(InMemorySessionService::new());
        let runner = build_test_runner(agent, session_service, "test-session-2")
            .await
            .unwrap();

        let content = Content::new("user").with_text("Hi");
        let raw_stream = runner
            .run_str("default-user", "test-session-2", content)
            .await
            .unwrap();

        let mut event_stream = AgentEventStream {
            inner: raw_stream,
            step: 1,
            done: false,
            detector: None,
        };

        let mut agent_events: Vec<AgentEvent> = Vec::new();
        while let Some(event) = event_stream.next().await {
            agent_events.push(event);
        }

        // Should have at least TextDelta and StepComplete
        assert!(!agent_events.is_empty(), "expected agent events");
        let text_events: Vec<_> = agent_events
            .iter()
            .filter(|e| matches!(e, AgentEvent::TextDelta(_)))
            .collect();
        assert!(!text_events.is_empty(), "expected at least one TextDelta");

        let has_step_complete = agent_events
            .iter()
            .any(|e| matches!(e, AgentEvent::StepComplete { .. }));
        assert!(has_step_complete, "expected StepComplete");
    }

    #[tokio::test]
    async fn test_agent_loop_empty_tools() {
        let mock = MockLlm::new("empty-tools-llm").with_response(LlmResponse::new(
            Content::new("assistant").with_text("No tools needed"),
        ));

        let mut builder = adk_agent::LlmAgentBuilder::new("no-tools-agent")
            .model(Arc::new(mock))
            .description("test agent with no tools")
            .max_iterations(3);
        builder = builder.tool(Arc::new(crate::agent::tools::read::ReadTool));
        let agent = builder.build().unwrap();

        let session_service = Arc::new(InMemorySessionService::new());
        let runner = build_test_runner(agent, session_service, "test-session-3")
            .await
            .unwrap();

        let content = Content::new("user").with_text("Hello");
        let mut stream = runner
            .run_str("default-user", "test-session-3", content)
            .await
            .unwrap();

        let mut event_count = 0;
        while let Some(result) = stream.next().await {
            assert!(result.is_ok());
            event_count += 1;
        }
        assert!(event_count > 0, "expected at least one event");
    }
}
