use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thinking_level: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction: Option<XyCompactionSettingsConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budgets: Option<ThinkingBudgets>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub steering_mode: Option<SteeringMode>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up_mode: Option<SteeringMode>,
}

pub use xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
#[derive(Default)]
pub struct ThinkingBudgets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimal: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<u64>,
}

impl From<&ThinkingBudgets> for xylitol_plugin_sdk::model::ThinkingBudgets {
    fn from(value: &ThinkingBudgets) -> Self {
        Self {
            minimal: value.minimal,
            low: value.low,
            medium: value.medium,
            high: value.high,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SteeringMode {
    All,
    #[default]
    #[serde(rename = "one-at-a-time")]
    OneAtATime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_settings_camelcase() {
        let settings = Settings {
            compaction: Some(XyCompactionSettingsConfig {
                enabled: Some(false),
                reserve_tokens: None,
                keep_recent_tokens: Some(32768),
                model: None,
                thinking_level: None,
            }),
            ..Default::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("keepRecentTokens"));
        assert!(!json.contains("keep_recent_tokens"));
    }

    #[test]
    fn unknown_legacy_keys_are_ignored() {
        let s: Settings = serde_json::from_str(
            r#"{"defaultModel":"x","extensions":["a"],"skills":[],"prompts":["p"]}"#,
        )
        .unwrap();
        assert_eq!(s, Settings::default());
    }
}
