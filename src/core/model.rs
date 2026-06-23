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

#[cfg(test)]
mod tests {
    use super::*;

    // ── ModelKind ───────────────────────────────────────────────────

    #[test]
    fn model_kind_from_provider_name_openai() {
        assert_eq!(
            ModelKind::from_provider_name("openai"),
            Some(ModelKind::OpenAi)
        );
    }

    #[test]
    fn model_kind_from_provider_name_anthropic() {
        assert_eq!(
            ModelKind::from_provider_name("anthropic"),
            Some(ModelKind::Anthropic)
        );
    }

    #[test]
    fn model_kind_from_provider_name_case_insensitive() {
        assert_eq!(
            ModelKind::from_provider_name("OpenAI"),
            Some(ModelKind::OpenAi)
        );
        assert_eq!(
            ModelKind::from_provider_name("ANTHROPIC"),
            Some(ModelKind::Anthropic)
        );
    }

    #[test]
    fn model_kind_from_provider_name_unknown() {
        assert_eq!(ModelKind::from_provider_name("google"), None);
        assert_eq!(ModelKind::from_provider_name(""), None);
    }

    #[test]
    fn model_kind_provider_name() {
        assert_eq!(ModelKind::OpenAi.provider_name(), "openai");
        assert_eq!(ModelKind::Anthropic.provider_name(), "anthropic");
    }

    #[test]
    fn model_kind_default_is_openai() {
        assert_eq!(ModelKind::default(), ModelKind::OpenAi);
    }

    #[test]
    fn model_kind_serde_round_trip() {
        let kinds = [ModelKind::OpenAi, ModelKind::Anthropic];
        for kind in &kinds {
            let json = serde_json::to_string(kind).unwrap();
            let deserialized: ModelKind = serde_json::from_str(&json).unwrap();
            assert_eq!(*kind, deserialized);
        }
    }

    #[test]
    fn model_kind_eq() {
        assert_eq!(ModelKind::OpenAi, ModelKind::OpenAi);
        assert_ne!(ModelKind::OpenAi, ModelKind::Anthropic);
    }

    // ── ModelConfig ─────────────────────────────────────────────────

    #[test]
    fn model_config_provider_name() {
        let config = ModelConfig {
            kind: ModelKind::Anthropic,
            api_key: "sk-test".into(),
            model: "claude-3".into(),
            base_url: None,
        };
        assert_eq!(config.provider_name(), "anthropic");
    }

    #[test]
    fn model_config_with_base_url() {
        let config = ModelConfig {
            kind: ModelKind::OpenAi,
            api_key: "sk-test".into(),
            model: "gpt-4".into(),
            base_url: Some("https://proxy.example.com/v1".into()),
        };
        assert_eq!(
            config.base_url.as_deref(),
            Some("https://proxy.example.com/v1")
        );
        assert_eq!(config.provider_name(), "openai");
    }

    // ── default_context_window_for ──────────────────────────────────

    #[test]
    fn default_context_window_openai() {
        assert_eq!(default_context_window_for(ModelKind::OpenAi), 128_000);
    }

    #[test]
    fn default_context_window_anthropic() {
        assert_eq!(default_context_window_for(ModelKind::Anthropic), 200_000);
    }

    // ── ResolvedProfile ─────────────────────────────────────────────

    #[test]
    fn resolved_profile_construct() {
        let config = ModelConfig {
            kind: ModelKind::OpenAi,
            api_key: "sk-test".into(),
            model: "gpt-4o".into(),
            base_url: None,
        };
        let profile = ResolvedProfile {
            model_config: config,
            system_prompt: Some("You are an AI".into()),
            allowed_tools: Some(vec!["read".into(), "write".into()]),
            max_iterations: 50,
            name: "default".into(),
        };
        assert_eq!(profile.name, "default");
        assert_eq!(profile.max_iterations, 50);
        assert_eq!(
            profile.allowed_tools.as_ref().map(|v| v.as_slice()),
            Some(&["read".to_string(), "write".to_string()][..])
        );
    }
}
