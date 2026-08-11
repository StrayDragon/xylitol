//! Settings types for user-controlled preferences (global / project / overrides).
//!
//! Only fields wired through bootstrap / product paths live here. Unknown JSON
//! keys are ignored (serde default) so older settings.json files still load.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// User-controlled settings that can live in global (~/.xylitol/settings.json)
/// or project (`<cwd>`/.xylitol/settings.json).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thinking_level: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<CompactionSettingsSchema>")]
    pub compaction: Option<XyCompactionSettingsConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budgets: Option<ThinkingBudgets>,

    /// Steering mode for follow-up messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steering_mode: Option<SteeringMode>,

    /// Follow-up mode for queue processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up_mode: Option<SteeringMode>,
}

pub use crate::infra::config::types::CompactionSettingsSchema;
/// Compaction behavior settings — domain serde type; schema twin in config.
pub use crate::protocol::compaction_config::XyCompactionSettingsConfig;

/// Thinking budget tokens per level.
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

impl From<&ThinkingBudgets> for crate::protocol::model::ThinkingBudgets {
    fn from(value: &ThinkingBudgets) -> Self {
        Self {
            minimal: value.minimal,
            low: value.low,
            medium: value.medium,
            high: value.high,
        }
    }
}

/// Steering / follow-up mode.
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
