/// LLM provider configuration and model registry.
///
/// Supports OpenAI-compatible (Response API) and Anthropic (Claude) providers.
/// The ModelKind enum locks to these two — no dynamic provider extension.
use adk_core::Llm;
use std::sync::Arc;

use crate::agent::r#loop::AgentError;

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
    pub(crate) fn build(&self) -> Result<Arc<dyn Llm>, AgentError> {
        match self.kind {
            ModelKind::OpenAi => {
                let mut cfg =
                    adk_model::openai::OpenAIConfig::new(self.api_key.clone(), self.model.clone());
                if let Some(ref base_url) = self.base_url {
                    cfg.base_url = Some(base_url.clone());
                }
                let client = adk_model::OpenAIClient::new(cfg)
                    .map_err(|e| AgentError::ConfigError(format!("OpenAI client: {e}")))?;
                Ok(Arc::new(client) as Arc<dyn Llm>)
            }
            ModelKind::Anthropic => {
                let mut cfg = adk_model::anthropic::AnthropicConfig::new(
                    self.api_key.clone(),
                    self.model.clone(),
                );
                if let Some(ref base_url) = self.base_url {
                    cfg.base_url = Some(base_url.clone());
                }
                let client = adk_model::AnthropicClient::new(cfg)
                    .map_err(|e| AgentError::ConfigError(format!("Anthropic client: {e}")))?;
                Ok(Arc::new(client) as Arc<dyn Llm>)
            }
            #[cfg(feature = "dev-fake-provider")]
            ModelKind::Fake => {
                let fake = crate::agent::provider::FakeProvider::new(
                    "__fake__",
                    vec![crate::agent::provider::ScenarioStep::text(
                        "Hello from __fake__ provider",
                    )],
                );
                Ok(Arc::new(fake) as Arc<dyn Llm>)
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
