//! Provider model kinds and configuration.
//!
//! Provides [`ModelKind`] (supported provider types) and [`ModelConfig`]
//! (connection parameters for building a provider instance).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Supported LLM provider kinds.
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

/// Connection parameters for building an LLM provider instance.
///
/// This is a pure data struct — the actual provider construction
/// lives in the [`agent`](crate::agent) layer where provider implementations
/// are available.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub kind: ModelKind,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
}

impl ModelConfig {
    /// Provider identifier for display purposes.
    pub fn provider_name(&self) -> &'static str {
        self.kind.provider_name()
    }
}

/// Fully resolved agent profile — runtime representation of an agent configuration.
///
/// Produced by [`AppConfig::resolve_profile`](crate::infra::config::types::AppConfig)
/// from config-level profile entries.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResolvedProfile {
    /// Agent-level model config.
    pub model_config: ModelConfig,
    /// System prompt override for this agent.
    pub system_prompt: Option<String>,
    /// Allowed tool names. `None` means all tools available.
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum ReAct loop iterations for this agent.
    pub max_iterations: u32,
    /// Profile name (for logging and diagnostics).
    pub name: String,
}

/// Default context window size for a given model kind.
pub fn default_context_window_for(kind: ModelKind) -> u64 {
    match kind {
        ModelKind::OpenAi => 128_000,
        ModelKind::Anthropic => 200_000,
        #[cfg(feature = "dev-fake-provider")]
        ModelKind::Fake => 8_000,
    }
}
