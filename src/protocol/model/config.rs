//! Provider model kinds and configuration.
//!
//! Provides [`XyModelKind`] (supported provider types) and [`XyModelConfig`]
//! (connection parameters for building a provider instance).
//!
//! JSON Schema for config files lives in `infra` (see c510); this module is
//! serde-only so domain stays free of schemars.

use serde::{Deserialize, Serialize};

/// Supported LLM provider kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum XyModelKind {
    #[serde(rename = "openai")]
    #[default]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "fake")]
    Fake,
}

impl XyModelKind {
    /// Parse from a provider name string (case-insensitive).
    pub fn from_provider_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAi),
            "anthropic" => Some(Self::Anthropic),
            "fake" => Some(Self::Fake),
            _ => None,
        }
    }

    /// Provider identifier for display and serialization.
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Fake => "fake",
        }
    }

    /// Default adapter API protocol family when YAML/`api` is omitted (c1598 / c1600).
    ///
    /// Keep in sync with infra `AdapterKind::default_for` → Display
    /// (`openai-responses` / `anthropic-messages`). Lives in protocol so agent
    /// manifest loading does not reach infra.
    pub fn default_adapter_api(self) -> &'static str {
        match self {
            Self::OpenAi | Self::Fake => "openai-responses",
            Self::Anthropic => "anthropic-messages",
        }
    }
}

/// Connection parameters for building an LLM provider instance.
///
/// This is a pure data struct — the actual provider construction
/// lives in the [`agent`](crate::agent) layer where provider implementations
/// are available.
#[derive(Debug, Clone)]
pub struct XyModelConfig {
    pub kind: XyModelKind,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
    /// Adapter API type, e.g. `openai-responses` or `openai-completions`.
    pub api: Option<String>,
    /// Named wire/thinking dialect profile (`generic` | `deepseek`); omit → generic.
    pub compat: Option<String>,
}

impl XyModelConfig {
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
    pub model_config: XyModelConfig,
    /// System prompt override for this agent.
    pub system_prompt: Option<String>,
    /// Allowed tool names. `None` means all tools available.
    pub allowed_tools: Option<Vec<String>>,
    /// Profile name (for logging and diagnostics).
    pub name: String,
}

/// Default context window size for a given model kind.
pub fn default_context_window_for(kind: XyModelKind) -> u64 {
    match kind {
        XyModelKind::OpenAi => 128_000,
        XyModelKind::Anthropic => 200_000,
        XyModelKind::Fake => 8_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── XyModelKind ───────────────────────────────────────────────────

    #[test]
    fn model_kind_from_provider_name_openai() {
        assert_eq!(
            XyModelKind::from_provider_name("openai"),
            Some(XyModelKind::OpenAi)
        );
    }

    #[test]
    fn model_kind_from_provider_name_anthropic() {
        assert_eq!(
            XyModelKind::from_provider_name("anthropic"),
            Some(XyModelKind::Anthropic)
        );
    }

    #[test]
    fn model_kind_from_provider_name_case_insensitive() {
        assert_eq!(
            XyModelKind::from_provider_name("OpenAI"),
            Some(XyModelKind::OpenAi)
        );
        assert_eq!(
            XyModelKind::from_provider_name("ANTHROPIC"),
            Some(XyModelKind::Anthropic)
        );
    }

    #[test]
    fn model_kind_from_provider_name_unknown() {
        assert_eq!(XyModelKind::from_provider_name("google"), None);
        assert_eq!(XyModelKind::from_provider_name(""), None);
    }

    #[test]
    fn model_kind_provider_name() {
        assert_eq!(XyModelKind::OpenAi.provider_name(), "openai");
        assert_eq!(XyModelKind::Anthropic.provider_name(), "anthropic");
    }

    #[test]
    fn model_kind_default_is_openai() {
        assert_eq!(XyModelKind::default(), XyModelKind::OpenAi);
    }

    #[test]
    fn model_kind_serde_round_trip() {
        let kinds = [XyModelKind::OpenAi, XyModelKind::Anthropic];
        for kind in &kinds {
            let json = serde_json::to_string(kind).unwrap();
            let deserialized: XyModelKind = serde_json::from_str(&json).unwrap();
            assert_eq!(*kind, deserialized);
        }
    }

    #[test]
    fn model_kind_eq() {
        assert_eq!(XyModelKind::OpenAi, XyModelKind::OpenAi);
        assert_ne!(XyModelKind::OpenAi, XyModelKind::Anthropic);
    }

    // ── XyModelConfig ─────────────────────────────────────────────────

    #[test]
    fn model_config_provider_name() {
        let config = XyModelConfig {
            kind: XyModelKind::Anthropic,
            api_key: "sk-test".into(),
            model: "claude-3".into(),
            base_url: None,
            api: None,
            compat: None,
        };
        assert_eq!(config.provider_name(), "anthropic");
    }

    #[test]
    fn model_config_with_base_url() {
        let config = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk-test".into(),
            model: "gpt-4".into(),
            base_url: Some("https://proxy.example.com/v1".into()),
            api: None,
            compat: None,
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
        assert_eq!(default_context_window_for(XyModelKind::OpenAi), 128_000);
    }

    #[test]
    fn default_context_window_anthropic() {
        assert_eq!(default_context_window_for(XyModelKind::Anthropic), 200_000);
    }

    // ── ResolvedProfile ─────────────────────────────────────────────

    #[test]
    fn resolved_profile_construct() {
        let config = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk-test".into(),
            model: "gpt-4o".into(),
            base_url: None,
            api: None,
            compat: None,
        };
        let profile = ResolvedProfile {
            model_config: config,
            system_prompt: Some("You are an AI".into()),
            allowed_tools: Some(vec!["read".into(), "write".into()]),
            name: "default".into(),
        };
        assert_eq!(profile.name, "default");
        assert_eq!(
            profile.allowed_tools.as_deref(),
            Some(&["read".to_string(), "write".to_string()][..])
        );
    }
}
