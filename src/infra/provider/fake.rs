use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use crate::domain::error::XyError;
use crate::domain::message::AgentMessage;
use crate::domain::types::{XyChunk, XyFinishReason, XyToolSchema};
use crate::runtime_protocol::{XyModel, XyStream};

// ---------------------------------------------------------------------------
// ScenarioStep
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ScenarioStep {
    Text(String),
    ToolCall { name: String, args: Value },
    ToolResult { name: String, result: Value },
    Delay(Duration),
    Error { message: String, retryable: bool },
}

impl ScenarioStep {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn tool_call(name: impl Into<String>, args: Value) -> Self {
        Self::ToolCall {
            name: name.into(),
            args,
        }
    }

    pub fn tool_result(name: impl Into<String>, result: Value) -> Self {
        Self::ToolResult {
            name: name.into(),
            result,
        }
    }

    pub fn delay(duration: Duration) -> Self {
        Self::Delay(duration)
    }

    pub fn error(message: impl Into<String>, retryable: bool) -> Self {
        Self::Error {
            message: message.into(),
            retryable,
        }
    }
}

// ---------------------------------------------------------------------------
// FakeProviderMode
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FakeProviderMode {
    #[default]
    Scenario,
    Cyclic,
}

// ---------------------------------------------------------------------------
// FakeProvider
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct FakeProvider {
    name: String,
    steps: Vec<ScenarioStep>,
    cursor: AtomicUsize,
    mode: FakeProviderMode,
}

impl FakeProvider {
    pub fn new(name: impl Into<String>, steps: Vec<ScenarioStep>) -> Self {
        Self {
            name: name.into(),
            steps,
            cursor: AtomicUsize::new(0),
            mode: FakeProviderMode::Scenario,
        }
    }

    pub fn builder(name: impl Into<String>) -> FakeProviderBuilder {
        FakeProviderBuilder::new(name)
    }

    pub fn with_mode(mut self, mode: FakeProviderMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn reset(&self) {
        self.cursor.store(0, Ordering::Release);
    }

    pub fn position(&self) -> usize {
        self.cursor.load(Ordering::Acquire)
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    fn fetch_index(&self) -> Option<usize> {
        let idx = self.cursor.fetch_add(1, Ordering::AcqRel);
        if idx < self.steps.len() {
            return Some(idx);
        }
        if self.mode == FakeProviderMode::Cyclic {
            loop {
                let cur = self.cursor.load(Ordering::Acquire);
                if cur < self.steps.len() {
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
}

struct StepOutcome {
    step: ScenarioStep,
    delay: Duration,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct FakeProviderBuilder {
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

    pub fn step(mut self, step: ScenarioStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn mode(mut self, mode: FakeProviderMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn build(self) -> FakeProvider {
        FakeProvider {
            name: self.name,
            steps: self.steps,
            cursor: AtomicUsize::new(0),
            mode: self.mode,
        }
    }
}

// ---------------------------------------------------------------------------
// XyModel implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl XyModel for FakeProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn generate_stream(
        &self,
        _messages: Vec<AgentMessage>,
        _tools: &[XyToolSchema],
        _stream: bool,
    ) -> Result<XyStream, XyError> {
        let outcome = match self.resolve_next() {
            Some(o) => o,
            None => return Ok(Box::pin(futures::stream::empty())),
        };

        if outcome.delay > Duration::ZERO {
            tokio::time::sleep(outcome.delay).await;
        }

        match outcome.step {
            ScenarioStep::Text(text) => Ok(Box::pin(futures::stream::iter(vec![
                Ok(XyChunk::TextDelta(text)),
                Ok(XyChunk::Done {
                    finish_reason: XyFinishReason::Stop,
                    usage: None,
                }),
            ]))),
            ScenarioStep::ToolCall { name, args } => {
                let id = format!("fake-call-{name}");
                Ok(Box::pin(futures::stream::iter(vec![
                    Ok(XyChunk::FunctionCall { name, args, id }),
                    Ok(XyChunk::Done {
                        finish_reason: XyFinishReason::Stop,
                        usage: None,
                    }),
                ])))
            }
            ScenarioStep::Error { message, .. } => {
                Err(XyError::Provider(anyhow::anyhow!("{message}")))
            }
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
        let mut stream = provider.generate_stream(vec![], &[], false).await.unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert!(matches!(chunk, XyChunk::TextDelta(t) if t == "Hello world"));
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
        let mut stream = provider.generate_stream(vec![], &[], false).await.unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert!(
            matches!(chunk, XyChunk::FunctionCall { name, .. } if name == "read"),
            "expected FunctionCall for 'read'"
        );
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

        let mut s1 = provider.generate_stream(vec![], &[], false).await.unwrap();
        let c1 = s1.next().await.unwrap().unwrap();
        assert!(matches!(c1, XyChunk::TextDelta(t) if t == "Hello!"));

        let mut s2 = provider.generate_stream(vec![], &[], false).await.unwrap();
        let c2 = s2.next().await.unwrap().unwrap();
        assert!(matches!(c2, XyChunk::FunctionCall { name, .. } if name == "search"));

        let mut s3 = provider.generate_stream(vec![], &[], false).await.unwrap();
        let c3 = s3.next().await.unwrap().unwrap();
        assert!(matches!(c3, XyChunk::TextDelta(t) if t == "Here are results."));

        let mut s4 = provider.generate_stream(vec![], &[], false).await.unwrap();
        assert!(s4.next().await.is_none());
    }

    #[tokio::test]
    async fn test_error_step() {
        let provider = FakeProvider::new(
            "test",
            vec![ScenarioStep::error("simulated rate limit", true)],
        );
        let result = provider.generate_stream(vec![], &[], false).await;
        assert!(result.is_err());
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
        let mut stream = provider.generate_stream(vec![], &[], false).await.unwrap();
        let _chunk = stream.next().await.unwrap().unwrap();
        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_cyclic_mode() {
        let provider = FakeProvider::new(
            "test",
            vec![ScenarioStep::text("a"), ScenarioStep::text("b")],
        )
        .with_mode(FakeProviderMode::Cyclic);

        for _ in 0..3 {
            let mut s1 = provider.generate_stream(vec![], &[], false).await.unwrap();
            let c1 = s1.next().await.unwrap().unwrap();
            assert!(matches!(c1, XyChunk::TextDelta(t) if t == "a"));

            let mut s2 = provider.generate_stream(vec![], &[], false).await.unwrap();
            let c2 = s2.next().await.unwrap().unwrap();
            assert!(matches!(c2, XyChunk::TextDelta(t) if t == "b"));
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

        let mut s1 = provider.generate_stream(vec![], &[], false).await.unwrap();
        let c1 = s1.next().await.unwrap().unwrap();
        assert!(matches!(c1, XyChunk::TextDelta(t) if t == "step1"));

        let mut s2 = provider.generate_stream(vec![], &[], false).await.unwrap();
        let c2 = s2.next().await.unwrap().unwrap();
        assert!(matches!(c2, XyChunk::FunctionCall { name, .. } if name == "tool1"));
    }

    #[tokio::test]
    async fn test_empty_scenario() {
        let provider = FakeProvider::new("empty", vec![]);
        let mut stream = provider.generate_stream(vec![], &[], false).await.unwrap();
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

        let mut s1 = provider.generate_stream(vec![], &[], false).await.unwrap();
        let c1 = s1.next().await.unwrap().unwrap();
        assert!(matches!(c1, XyChunk::FunctionCall { name, .. } if name == "read"));

        let mut s2 = provider.generate_stream(vec![], &[], false).await.unwrap();
        let c2 = s2.next().await.unwrap().unwrap();
        assert!(matches!(c2, XyChunk::TextDelta(t) if t == "Got the data"));

        let mut s3 = provider.generate_stream(vec![], &[], false).await.unwrap();
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
        let mut stream = provider.generate_stream(vec![], &[], false).await.unwrap();
        let _chunk = stream.next().await.unwrap().unwrap();
        assert!(start.elapsed() >= Duration::from_millis(60));
    }

    #[tokio::test]
    async fn test_reset() {
        let provider = FakeProvider::new("test", vec![ScenarioStep::text("only")]);

        let mut s1 = provider.generate_stream(vec![], &[], false).await.unwrap();
        assert!(s1.next().await.is_some());

        let mut s2 = provider.generate_stream(vec![], &[], false).await.unwrap();
        assert!(s2.next().await.is_none());

        provider.reset();
        let mut s3 = provider.generate_stream(vec![], &[], false).await.unwrap();
        assert!(s3.next().await.is_some());
    }
}
