//! Domain-facing Fake provider — the bridge fake exposed through the unified
//! adapter shell (pa1/pa6, r38: same assembly path as vendor models).

use std::sync::Arc;

use crate::infra::provider::adapter::AdapterXyModel;
use crate::protocol::ports::XyModel;

pub use xylitol_ai_bridge::fake::{FakeProvider, ScenarioStep};

/// Build a scenario-driven fake `XyModel` via the single-layer adapter shell.
pub fn fake_xy_model(name: impl Into<String>, steps: Vec<ScenarioStep>) -> Arc<dyn XyModel> {
    fake_xy_model_of(FakeProvider::new(name, steps))
}

/// Expose an already-configured [`FakeProvider`] (builder / custom mode) as an
/// `XyModel` via the single-layer adapter shell.
pub fn fake_xy_model_of(provider: FakeProvider) -> Arc<dyn XyModel> {
    Arc::new(AdapterXyModel::new(Arc::new(provider)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::model::XyChunk;
    use crate::protocol::ports::XyGenerateOptions;
    use futures::StreamExt;

    #[tokio::test]
    async fn fake_xy_model_text() {
        let model = fake_xy_model("test", vec![ScenarioStep::text("Hello world")]);
        let mut stream = model
            .generate_stream(vec![], &[], false, XyGenerateOptions::default())
            .await
            .unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert!(matches!(chunk, XyChunk::TextDelta(t) if t == "Hello world"));
    }

    #[tokio::test]
    async fn builder_builds_bridge_then_shell() {
        let bridge = FakeProvider::builder("b")
            .step(ScenarioStep::text("x"))
            .build();
        let model = fake_xy_model_of(bridge);
        assert_eq!(XyModel::name(model.as_ref()), "b");
    }
}
