//! Domain-facing Fake provider — wraps xylitol-ai-bridge Fake (c1030).

use async_trait::async_trait;

use crate::infra::provider::map::{to_bridge_tools, to_xy_error, to_xy_stream};
use crate::protocol::error::XyError;
use crate::protocol::message::LlmMessage;
use crate::protocol::model::XyToolSchema;
use crate::protocol::ports::{XyGenerateOptions, XyModel, XyStream};

pub use xylitol_ai_bridge::fake::{
    FakeProvider as AiBridgeFakeProvider, FakeProviderBuilder, FakeProviderMode, ScenarioStep,
};

/// Scenario-based offline model used by BDD / unit tests.
pub struct FakeProvider {
    inner: AiBridgeFakeProvider,
}

impl FakeProvider {
    pub fn new(name: impl Into<String>, steps: Vec<ScenarioStep>) -> Self {
        Self {
            inner: AiBridgeFakeProvider::new(name, steps),
        }
    }

    pub fn builder(name: impl Into<String>) -> FakeProviderBuilder {
        AiBridgeFakeProvider::builder(name)
    }

    pub fn with_mode(mut self, mode: FakeProviderMode) -> Self {
        self.inner = self.inner.with_mode(mode);
        self
    }

    pub fn reset(&self) {
        self.inner.reset();
    }
}

impl From<AiBridgeFakeProvider> for FakeProvider {
    fn from(inner: AiBridgeFakeProvider) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl XyModel for FakeProvider {
    fn name(&self) -> &str {
        xylitol_ai_bridge::fake::AiBridgeModel::name(&self.inner)
    }

    async fn generate_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        stream: bool,
        _options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        let bridge_tools = to_bridge_tools(tools);
        let bridge_stream = xylitol_ai_bridge::fake::AiBridgeModel::generate_stream(
            &self.inner,
            messages,
            &bridge_tools,
            stream,
        )
        .await
        .map_err(to_xy_error)?;
        Ok(to_xy_stream(bridge_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use xylitol_ai_bridge::fake::AiBridgeModel;

    #[tokio::test]
    async fn fake_xy_model_text() {
        let provider = FakeProvider::new("test", vec![ScenarioStep::text("Hello world")]);
        let mut stream = XyModel::generate_stream(
            &provider,
            vec![],
            &[],
            false,
            crate::protocol::ports::XyGenerateOptions::default(),
        )
        .await
        .unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert!(matches!(
            chunk,
            crate::protocol::model::XyChunk::TextDelta(t) if t == "Hello world"
        ));
    }

    #[tokio::test]
    async fn builder_builds_bridge_then_wrap() {
        let bridge = FakeProvider::builder("b")
            .step(ScenarioStep::text("x"))
            .build();
        let provider = FakeProvider::from(bridge);
        assert_eq!(XyModel::name(&provider), "b");
        let _ = AiBridgeModel::name(&provider.inner);
    }
}
