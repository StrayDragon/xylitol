use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{EnumString, IntoStaticStr};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, IntoStaticStr, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum XyModelKind {
    #[serde(rename = "openai")]
    #[strum(serialize = "openai")]
    #[default]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "fake")]
    Fake,
}

impl XyModelKind {
    pub fn from_provider_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    pub fn provider_name(&self) -> &'static str {
        self.into()
    }

    pub fn default_adapter_api(self) -> &'static str {
        match self {
            Self::OpenAi | Self::Fake => "openai-responses",
            Self::Anthropic => "anthropic-messages",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum XyModality {
    Text,
    Image,
    Audio,
    Video,
}

pub fn default_input_modalities(kind: XyModelKind) -> Vec<XyModality> {
    match kind {
        XyModelKind::OpenAi | XyModelKind::Anthropic => {
            vec![XyModality::Text, XyModality::Image]
        }
        XyModelKind::Fake => vec![XyModality::Text],
    }
}

#[derive(Debug, Clone)]
pub struct XyModelConfig {
    pub kind: XyModelKind,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api: Option<String>,
    pub compat: Option<String>,
}

impl XyModelConfig {
    pub fn provider_name(&self) -> &'static str {
        self.kind.provider_name()
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub model_id: String,
    pub system_prompt: Option<String>,
    pub tools: Vec<String>,
}

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

    #[test]
    fn default_context_window_openai() {
        assert_eq!(default_context_window_for(XyModelKind::OpenAi), 128_000);
    }

    #[test]
    fn default_context_window_anthropic() {
        assert_eq!(default_context_window_for(XyModelKind::Anthropic), 200_000);
    }

    #[test]
    fn resolved_profile_construct() {
        let profile = ResolvedProfile {
            model_id: "gpt-4o".into(),
            system_prompt: Some("You are an AI".into()),
            tools: vec!["read".into()],
        };
        assert_eq!(profile.model_id, "gpt-4o");
        assert_eq!(profile.system_prompt.as_deref(), Some("You are an AI"));
        assert_eq!(profile.tools, vec!["read".to_string()]);
    }
}
