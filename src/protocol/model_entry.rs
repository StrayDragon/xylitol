//! YAML wire shape for a single model alias entry (`models.models.<alias>`).
//!
//! SSOT for config-loaded model entries (`models.models.*` and `compaction.model`).
//! `snake_case` keys (no `rename_all`) so YAML anchor/alias
//! reuse works across `models` and `compaction.model`.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::protocol::model::XyModelKind;

fn default_thinking() -> bool {
    true
}

/// Config-file model entry (models map + compaction task model).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema)]
pub struct XyModelEntryConfig {
    #[schemars(with = "String")]
    pub provider: XyModelKind,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub compat: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
    #[serde(default = "default_thinking")]
    pub thinking: bool,
    #[serde(default)]
    pub thinking_levels: Option<Vec<String>>,
    #[serde(default)]
    pub thinking_level_map: Option<HashMap<String, Option<String>>>,
    #[serde(default)]
    pub context_window: u64,
    #[serde(default)]
    pub tokenizer: Option<String>,
}

impl XyModelEntryConfig {
    /// Per-model API key after config/secret render; empty when omitted (m17).
    pub fn normalized_api_key(&self) -> String {
        match &self.api_key {
            Some(k) if !k.is_empty() => k.clone(),
            _ => String::new(),
        }
    }

    /// Convert to runtime [`XyModelConfig`](crate::protocol::model::XyModelConfig).
    pub fn to_model_config(&self) -> crate::protocol::model::XyModelConfig {
        crate::protocol::model::XyModelConfig {
            kind: self.provider,
            api_key: self.normalized_api_key(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
            api: self.api.clone(),
            compat: self.compat.clone(),
        }
    }

    /// Resolve thinking support set + level map.
    pub fn resolve_thinking_config(
        &self,
    ) -> Result<(Vec<String>, crate::protocol::model::ThinkingLevelMap), String> {
        let levels = crate::protocol::model::resolve_configured_levels(
            self.thinking,
            self.thinking_levels.as_deref(),
        )
        .map_err(|e| e.to_string())?;
        let map = self.thinking_level_map.clone().unwrap_or_default();
        crate::protocol::model::validate_thinking_level_map(&map, &levels)
            .map_err(|e| e.to_string())?;
        Ok((levels, map))
    }

    /// Build registry [`XyModelMeta`](crate::protocol::model::XyModelMeta) for alias `id`.
    pub fn to_model_meta(&self, id: &str) -> Result<crate::protocol::model::XyModelMeta, String> {
        use crate::protocol::model::{XyModelMeta, default_context_window_for};

        let config = self.to_model_config();
        let (thinking_levels, thinking_level_map) = self.resolve_thinking_config()?;
        let context_window = if self.context_window > 0 {
            self.context_window
        } else {
            default_context_window_for(config.kind)
        };
        Ok(XyModelMeta {
            id: id.to_string(),
            config,
            display_name: id.to_string(),
            thinking: self.thinking,
            context_window,
            api: self.api.clone().unwrap_or_default(),
            provider: self.provider.provider_name().to_string(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels,
            thinking_level_map,
        })
    }

    /// Ephemeral meta for task-scoped models (not registered in ModelRegistry).
    pub fn to_ephemeral_meta(&self) -> Result<crate::protocol::model::XyModelMeta, String> {
        self.to_model_meta(&self.model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_unknown_fields_in_json() {
        let json = r#"{
            "provider": "fake",
            "model": "summary",
            "extra": "dropped"
        }"#;
        let entry: XyModelEntryConfig = serde_json::from_str(json).expect("parse");
        assert_eq!(entry.model, "summary");
    }

    #[test]
    fn accepts_snake_case_yaml_keys() {
        let yaml = r"
provider: fake
model: summary-bot
base_url: https://example.com/v1
api_key: sk-test
thinking_levels: [off, high]
context_window: 64000
";
        let entry: XyModelEntryConfig = yaml_serde::from_str(yaml).expect("snake_case yaml");
        assert_eq!(entry.model, "summary-bot");
        assert_eq!(entry.base_url.as_deref(), Some("https://example.com/v1"));
        assert_eq!(entry.api_key.as_deref(), Some("sk-test"));
        assert_eq!(
            entry.thinking_levels,
            Some(vec!["off".into(), "high".into()])
        );
        assert_eq!(entry.context_window, 64_000);
    }

    #[test]
    fn thinking_validation_rejects_blank_level() {
        let entry = XyModelEntryConfig {
            provider: XyModelKind::Fake,
            model: "s".into(),
            thinking: true,
            thinking_levels: Some(vec!["".into()]),
            ..Default::default()
        };
        assert!(entry.resolve_thinking_config().is_err());
    }
}
