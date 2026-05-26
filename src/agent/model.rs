/// LLM provider configuration and model registry.
///
/// Supports OpenAI-compatible and Anthropic providers via direct HTTP integration.
use std::sync::Arc;

use crate::agent::r#loop::AgentError;
use crate::agent::traits::XyModel;

/// Supported model provider kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelKind {
    OpenAi,
    Anthropic,
    #[cfg(feature = "dev-fake-provider")]
    Fake,
}

/// Configuration for building an LLM provider.
#[derive(Debug, Clone)]
pub(crate) struct ModelConfig {
    pub(crate) kind: ModelKind,
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) base_url: Option<String>,
}

impl ModelConfig {
    /// Build the LLM provider from this configuration.
    pub(crate) fn build(&self) -> Result<Arc<dyn XyModel>, AgentError> {
        match self.kind {
            ModelKind::OpenAi => {
                let provider = crate::agent::provider::openai::OpenAIProvider::new(
                    self.api_key.clone(),
                    self.model.clone(),
                    self.base_url.clone(),
                );
                Ok(Arc::new(provider) as Arc<dyn XyModel>)
            }
            ModelKind::Anthropic => {
                let provider = crate::agent::provider::anthropic::AnthropicProvider::new(
                    self.api_key.clone(),
                    self.model.clone(),
                    self.base_url.clone(),
                );
                Ok(Arc::new(provider) as Arc<dyn XyModel>)
            }
            #[cfg(feature = "dev-fake-provider")]
            ModelKind::Fake => {
                let fake = crate::agent::provider::FakeProvider::new(
                    "__fake__",
                    vec![crate::agent::provider::ScenarioStep::text(
                        "Hello from __fake__ provider",
                    )],
                );
                Ok(Arc::new(fake) as Arc<dyn XyModel>)
            }
        }
    }

    /// Provider identifier for display purposes.
    pub(crate) fn provider_name(&self) -> &'static str {
        match self.kind {
            ModelKind::OpenAi => "openai",
            ModelKind::Anthropic => "anthropic",
            #[cfg(feature = "dev-fake-provider")]
            ModelKind::Fake => "fake",
        }
    }
}
