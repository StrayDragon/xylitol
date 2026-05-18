use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use adk_core::{Content, Llm, LlmRequest, LlmResponse, LlmResponseStream, Part};
use async_trait::async_trait;
use serde_json::Value;

// ---------------------------------------------------------------------------
// ScenarioStep
// ---------------------------------------------------------------------------

/// A single step in a fake-provider scenario.
///
/// Each call to [`FakeProvider::generate_content`] consumes one or more
/// steps and returns the corresponding response.
#[derive(Debug, Clone)]
pub(crate) enum ScenarioStep {
    /// Return a text response.
    Text(String),
    /// Return a tool-call response (Part::FunctionCall).
    ToolCall { name: String, args: Value },
    /// Assert that the conversation history contains this tool result.
    ///
    /// This step is **consumed without producing a response** — the
    /// provider advances to the next step and returns that instead.
    ToolResult { name: String, result: Value },
    /// Sleep for the given duration, then continue to the next step.
    Delay(Duration),
    /// Return an error to the caller.
    Error {
        message: String,
        code: &'static str,
        retryable: bool,
    },
}

impl ScenarioStep {
    /// Shortcut: text step.
    pub(crate) fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Shortcut: tool-call step.
    pub(crate) fn tool_call(name: impl Into<String>, args: Value) -> Self {
        Self::ToolCall {
            name: name.into(),
            args,
        }
    }

    /// Shortcut: tool-result step.
    pub(crate) fn tool_result(name: impl Into<String>, result: Value) -> Self {
        Self::ToolResult {
            name: name.into(),
            result,
        }
    }

    /// Shortcut: delay step.
    pub(crate) fn delay(duration: Duration) -> Self {
        Self::Delay(duration)
    }

    /// Shortcut: error step.
    pub(crate) fn error(err: adk_core::AdkError) -> Self {
        Self::Error {
            message: err.message.clone(),
            code: err.code,
            retryable: err.is_retryable(),
        }
    }
}

// ---------------------------------------------------------------------------
// FakeProviderMode
// ---------------------------------------------------------------------------

/// How the provider behaves when all steps have been consumed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FakeProviderMode {
    /// Return an empty response stream when steps exhausted.
    #[default]
    Scenario,
    /// Wrap back to step 0 and repeat.
    Cyclic,
}

// ---------------------------------------------------------------------------
// FakeProvider
// ---------------------------------------------------------------------------

/// A fake LLM provider driven by a scripted scenario.
///
/// Implements [`adk_core::Llm`] so it can be plugged directly into
/// `LlmAgentBuilder` / `Runner` in place of a real model.
///
/// # Example
///
/// ```rust,ignore
/// use xylitol::agent::provider::{FakeProvider, ScenarioStep};
///
/// let fake = FakeProvider::builder("test-llm")
///     .step(ScenarioStep::text("Hello!"))
///     .step(ScenarioStep::tool_call("read", serde_json::json!({"path": "/tmp"})))
///     .build();
/// ```
#[derive(Debug)]
pub(crate) struct FakeProvider {
    name: String,
    steps: Vec<ScenarioStep>,
    cursor: AtomicUsize,
    mode: FakeProviderMode,
}

impl FakeProvider {
    /// Create a provider that runs through the given steps once.
    pub(crate) fn new(name: impl Into<String>, steps: Vec<ScenarioStep>) -> Self {
        Self {
            name: name.into(),
            steps,
            cursor: AtomicUsize::new(0),
            mode: FakeProviderMode::Scenario,
        }
    }

    /// Start building a FakeProvider with the builder API.
    pub(crate) fn builder(name: impl Into<String>) -> FakeProviderBuilder {
        FakeProviderBuilder::new(name)
    }

    /// Set the mode (scenario or cyclic).
    pub(crate) fn with_mode(mut self, mode: FakeProviderMode) -> Self {
        self.mode = mode;
        self
    }

    /// Reset the cursor to the beginning.
    pub(crate) fn reset(&self) {
        self.cursor.store(0, Ordering::Release);
    }

    /// Return the current cursor position.
    pub(crate) fn position(&self) -> usize {
        self.cursor.load(Ordering::Acquire)
    }

    /// Return the total number of steps.
    pub(crate) fn len(&self) -> usize {
        self.steps.len()
    }

    /// Return true if no steps are configured.
    pub(crate) fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Fetch the next step index, handling cyclic wrap.
    ///
    /// Returns the index into `self.steps`, or `None` if exhausted
    /// (only possible in Scenario mode).
    fn fetch_index(&self) -> Option<usize> {
        let idx = self.cursor.fetch_add(1, Ordering::AcqRel);
        if idx < self.steps.len() {
            return Some(idx);
        }
        if self.mode == FakeProviderMode::Cyclic {
            // CAS loop: only the thread that sees itself past the end
            // resets the cursor. Other threads re-read the new value.
            loop {
                let cur = self.cursor.load(Ordering::Acquire);
                if cur < self.steps.len() {
                    // Another thread already wrapped — take the step.
                    return Some(self.cursor.fetch_add(1, Ordering::AcqRel));
                }
                if self
                    .cursor
                    .compare_exchange(cur, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    return Some(0);
                }
            }
        }
        None
    }

    /// Resolve the next producing step and any accumulated delay.
    ///
    /// Skips `ToolResult` steps and accumulates `Delay` step durations
    /// until a producing step (`Text`, `ToolCall`, `Error`) or exhaustion.
    fn resolve_next(&self) -> Option<StepOutcome> {
        let mut accumulated = Duration::ZERO;

        loop {
            let idx = self.fetch_index()?;
            match &self.steps[idx] {
                ScenarioStep::Delay(d) => {
                    accumulated += *d;
                    continue;
                }
                ScenarioStep::ToolResult { .. } => continue,
                ScenarioStep::Text(_)
                | ScenarioStep::ToolCall { .. }
                | ScenarioStep::Error { .. } => {
                    return Some(StepOutcome {
                        step: self.steps[idx].clone(),
                        delay: accumulated,
                    });
                }
            }
        }
    }

    fn text_response(text: &str) -> LlmResponse {
        LlmResponse::new(Content::new("assistant").with_text(text.to_owned()))
    }

    fn tool_call_response(name: &str, args: &Value) -> LlmResponse {
        let content = Content {
            role: "assistant".into(),
            parts: vec![Part::FunctionCall {
                name: name.to_owned(),
                args: args.clone(),
                id: Some(format!("fake-call-{name}")),
                thought_signature: None,
            }],
        };
        LlmResponse::new(content)
    }
}

/// Outcome of resolving the next producing step.
struct StepOutcome {
    step: ScenarioStep,
    delay: Duration,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`FakeProvider`].
#[derive(Debug)]
pub(crate) struct FakeProviderBuilder {
    name: String,
    steps: Vec<ScenarioStep>,
    mode: FakeProviderMode,
}

impl FakeProviderBuilder {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
            mode: FakeProviderMode::Scenario,
        }
    }

    /// Append a scenario step.
    pub(crate) fn step(mut self, step: ScenarioStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Set the mode (default: scenario).
    pub(crate) fn mode(mut self, mode: FakeProviderMode) -> Self {
        self.mode = mode;
        self
    }

    /// Build the [`FakeProvider`].
    pub(crate) fn build(self) -> FakeProvider {
        FakeProvider {
            name: self.name,
            steps: self.steps,
            cursor: AtomicUsize::new(0),
            mode: self.mode,
        }
    }
}

// ---------------------------------------------------------------------------
// Llm trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl Llm for FakeProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn generate_content(
        &self,
        _req: LlmRequest,
        _stream: bool,
    ) -> Result<LlmResponseStream, adk_core::AdkError> {
        let outcome = match self.resolve_next() {
            Some(o) => o,
            None => return Ok(Box::pin(futures::stream::empty())),
        };

        // Apply accumulated delay before producing a response.
        if outcome.delay > Duration::ZERO {
            tokio::time::sleep(outcome.delay).await;
        }

        match outcome.step {
            ScenarioStep::Text(text) => {
                let response = Self::text_response(&text);
                Ok(Box::pin(futures::stream::once(async move { Ok(response) })))
            }
            ScenarioStep::ToolCall { name, args } => {
                let response = Self::tool_call_response(&name, &args);
                Ok(Box::pin(futures::stream::once(async move { Ok(response) })))
            }
            ScenarioStep::Error {
                message,
                code,
                retryable,
            } => Err(adk_core::AdkError::new(
                adk_core::ErrorComponent::Model,
                if retryable {
                    adk_core::ErrorCategory::Unavailable
                } else {
                    adk_core::ErrorCategory::Internal
                },
                code,
                message,
            )),
            // Non-producing steps are never returned by resolve_next().
            ScenarioStep::ToolResult { .. } | ScenarioStep::Delay(_) => {
                Ok(Box::pin(futures::stream::empty()))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_text_step() {
        let provider = FakeProvider::new("test", vec![ScenarioStep::text("Hello world")]);
        let req = LlmRequest::new("test", vec![]);
        let mut stream = provider.generate_content(req, false).await.unwrap();
        let response = stream.next().await.unwrap().unwrap();
        let text = response.content.unwrap().parts.into_iter().find_map(|p| {
            if let Part::Text { text } = p {
                Some(text)
            } else {
                None
            }
        });
        assert_eq!(text.as_deref(), Some("Hello world"));
    }

    #[tokio::test]
    async fn test_tool_call_step() {
        let provider = FakeProvider::new(
            "test",
            vec![ScenarioStep::tool_call(
                "read",
                serde_json::json!({"path": "/tmp"}),
            )],
        );
        let req = LlmRequest::new("test", vec![]);
        let mut stream = provider.generate_content(req, false).await.unwrap();
        let response = stream.next().await.unwrap().unwrap();
        let has_call = response.content.as_ref().is_some_and(|c| {
            c.parts
                .iter()
                .any(|p| matches!(p, Part::FunctionCall { name, .. } if name == "read"))
        });
        assert!(has_call, "expected FunctionCall for 'read'");
    }

    #[tokio::test]
    async fn test_multi_turn_scenario() {
        let provider = FakeProvider::new(
            "test",
            vec![
                ScenarioStep::text("Hello!"),
                ScenarioStep::tool_call("search", serde_json::json!({"q": "rust"})),
                ScenarioStep::text("Here are results."),
            ],
        );
        let req = LlmRequest::new("test", vec![]);

        // Turn 1
        let mut s1 = provider.generate_content(req.clone(), false).await.unwrap();
        let r1 = s1.next().await.unwrap().unwrap();
        assert!(r1.content.as_ref().is_some_and(|c| {
            c.parts
                .iter()
                .any(|p| matches!(p, Part::Text { text } if text == "Hello!"))
        }));

        // Turn 2
        let mut s2 = provider.generate_content(req.clone(), false).await.unwrap();
        let r2 = s2.next().await.unwrap().unwrap();
        assert!(r2.content.as_ref().is_some_and(|c| {
            c.parts
                .iter()
                .any(|p| matches!(p, Part::FunctionCall { name, .. } if name == "search"))
        }));

        // Turn 3
        let mut s3 = provider.generate_content(req.clone(), false).await.unwrap();
        let r3 = s3.next().await.unwrap().unwrap();
        assert!(r3.content.as_ref().is_some_and(|c| {
            c.parts
                .iter()
                .any(|p| matches!(p, Part::Text { text } if text == "Here are results."))
        }));

        // Turn 4 — exhausted
        let mut s4 = provider.generate_content(req, false).await.unwrap();
        assert!(s4.next().await.is_none());
    }

    #[tokio::test]
    async fn test_error_step() {
        let provider = FakeProvider::new(
            "test",
            vec![ScenarioStep::error(adk_core::AdkError::rate_limited(
                adk_core::ErrorComponent::Model,
                "fake.rate_limited",
                "simulated rate limit",
            ))],
        );
        let req = LlmRequest::new("test", vec![]);
        let result = provider.generate_content(req, false).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.is_retryable());
        assert_eq!(err.code, "fake.rate_limited");
    }

    #[tokio::test]
    async fn test_delay_step() {
        use std::time::Instant;
        let provider = FakeProvider::new(
            "test",
            vec![
                ScenarioStep::delay(Duration::from_millis(50)),
                ScenarioStep::text("after delay"),
            ],
        );
        let start = Instant::now();
        let req = LlmRequest::new("test", vec![]);
        let mut stream = provider.generate_content(req, false).await.unwrap();
        let _response = stream.next().await.unwrap().unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_cyclic_mode() {
        let provider = FakeProvider::new(
            "test",
            vec![ScenarioStep::text("a"), ScenarioStep::text("b")],
        )
        .with_mode(FakeProviderMode::Cyclic);

        let req = LlmRequest::new("test", vec![]);

        // Cycle through multiple times
        for _ in 0..3 {
            let mut s1 = provider.generate_content(req.clone(), false).await.unwrap();
            let r1 = s1.next().await.unwrap().unwrap();
            assert!(r1.content.as_ref().is_some_and(|c| {
                c.parts
                    .iter()
                    .any(|p| matches!(p, Part::Text { text } if text == "a"))
            }));

            let mut s2 = provider.generate_content(req.clone(), false).await.unwrap();
            let r2 = s2.next().await.unwrap().unwrap();
            assert!(r2.content.as_ref().is_some_and(|c| {
                c.parts
                    .iter()
                    .any(|p| matches!(p, Part::Text { text } if text == "b"))
            }));
        }
    }

    #[tokio::test]
    async fn test_builder_api() {
        let provider = FakeProvider::builder("builder-llm")
            .step(ScenarioStep::text("step1"))
            .step(ScenarioStep::tool_call(
                "tool1",
                serde_json::json!({"x": 1}),
            ))
            .mode(FakeProviderMode::Scenario)
            .build();

        assert_eq!(provider.name(), "builder-llm");
        assert_eq!(provider.len(), 2);

        let req = LlmRequest::new("test", vec![]);
        let mut s1 = provider.generate_content(req.clone(), false).await.unwrap();
        let r1 = s1.next().await.unwrap().unwrap();
        assert!(r1.content.as_ref().is_some_and(|c| {
            c.parts
                .iter()
                .any(|p| matches!(p, Part::Text { text } if text == "step1"))
        }));

        let mut s2 = provider.generate_content(req, false).await.unwrap();
        let r2 = s2.next().await.unwrap().unwrap();
        assert!(r2.content.as_ref().is_some_and(|c| {
            c.parts
                .iter()
                .any(|p| matches!(p, Part::FunctionCall { name, .. } if name == "tool1"))
        }));
    }

    #[tokio::test]
    async fn test_empty_scenario() {
        let provider = FakeProvider::new("empty", vec![]);
        let req = LlmRequest::new("test", vec![]);
        let mut stream = provider.generate_content(req, false).await.unwrap();
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_tool_result_skips_and_returns_next() {
        let provider = FakeProvider::new(
            "test",
            vec![
                ScenarioStep::tool_call("read", serde_json::json!({"path": "/tmp"})),
                ScenarioStep::tool_result("read", serde_json::json!({"content": "data"})),
                ScenarioStep::text("Got the data"),
            ],
        );

        let req = LlmRequest::new("test", vec![]);

        // Turn 1: tool call
        let mut s1 = provider.generate_content(req.clone(), false).await.unwrap();
        let r1 = s1.next().await.unwrap().unwrap();
        assert!(r1.content.as_ref().is_some_and(|c| {
            c.parts
                .iter()
                .any(|p| matches!(p, Part::FunctionCall { name, .. } if name == "read"))
        }));

        // Turn 2: tool_result is skipped, returns text step
        let mut s2 = provider.generate_content(req.clone(), false).await.unwrap();
        let r2 = s2.next().await.unwrap().unwrap();
        assert!(r2.content.as_ref().is_some_and(|c| {
            c.parts
                .iter()
                .any(|p| matches!(p, Part::Text { text } if text == "Got the data"))
        }));

        // Turn 3: exhausted
        let mut s3 = provider.generate_content(req, false).await.unwrap();
        assert!(s3.next().await.is_none());
    }

    #[tokio::test]
    async fn test_accumulated_delays() {
        use std::time::Instant;
        let provider = FakeProvider::new(
            "test",
            vec![
                ScenarioStep::delay(Duration::from_millis(30)),
                ScenarioStep::delay(Duration::from_millis(30)),
                ScenarioStep::text("after accumulated delays"),
            ],
        );

        let start = Instant::now();
        let req = LlmRequest::new("test", vec![]);
        let mut stream = provider.generate_content(req, false).await.unwrap();
        let _response = stream.next().await.unwrap().unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(60));
    }

    #[tokio::test]
    async fn test_reset() {
        let provider = FakeProvider::new("test", vec![ScenarioStep::text("only")]);

        let req = LlmRequest::new("test", vec![]);

        // Consume the only step
        let mut s1 = provider.generate_content(req.clone(), false).await.unwrap();
        assert!(s1.next().await.is_some());

        // Now exhausted
        let mut s2 = provider.generate_content(req.clone(), false).await.unwrap();
        assert!(s2.next().await.is_none());

        // Reset and try again
        provider.reset();
        let mut s3 = provider.generate_content(req, false).await.unwrap();
        assert!(s3.next().await.is_some());
    }
}
