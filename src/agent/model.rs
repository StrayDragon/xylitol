/// LLM provider configuration and model registry.
///
/// Supports OpenAI-compatible and Anthropic providers via direct HTTP integration.
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::agent::traits::XyModel;
use schemars::JsonSchema;

/// Supported LLM providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    #[serde(rename = "openai")]
    #[default]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[cfg(feature = "dev-fake-provider")]
    #[serde(rename = "fake")]
    Fake,
}

impl ModelKind {
    /// Parse from a provider name string (case-insensitive).
    pub fn from_provider_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            #[cfg(feature = "dev-fake-provider")]
            "fake" => Some(Self::Fake),
            _ => None,
        }
    }

    /// Provider identifier for display and serialization.
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            #[cfg(feature = "dev-fake-provider")]
            Self::Fake => "fake",
        }
    }
}

/// Configuration for building an LLM provider instance.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub kind: ModelKind,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}

impl ModelConfig {
    /// Build the LLM provider from this configuration.
    pub fn build(&self) -> Result<Arc<dyn XyModel>, String> {
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
    pub fn provider_name(&self) -> &'static str {
        self.kind.provider_name()
    }
}
