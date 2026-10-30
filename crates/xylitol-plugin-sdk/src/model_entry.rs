use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::{XyModality, XyModelKind, default_input_modalities};

fn default_thinking() -> bool {
    true
}

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
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub prompt_cache: Option<crate::model::XyPromptCacheConfig>,
    #[serde(default)]
    pub input_modalities: Option<Vec<XyModality>>,
}

impl XyModelEntryConfig {
    pub fn normalized_api_key(&self) -> String {
        match &self.api_key {
            Some(k) if !k.is_empty() => k.clone(),
            _ => String::new(),
        }
    }

    pub fn to_model_config(&self) -> crate::model::XyModelConfig {
        crate::model::XyModelConfig {
            kind: self.provider,
            api_key: self.normalized_api_key(),
            model: self.model.clone(),
            base_url: self.base_url.clone(),
            api: self.api.clone(),
            compat: self.compat.clone(),
            prompt_cache: self.prompt_cache,
        }
    }

    pub fn resolve_thinking_config(
        &self,
    ) -> Result<(Vec<String>, crate::model::ThinkingLevelMap), String> {
        let levels =
            crate::model::resolve_configured_levels(self.thinking, self.thinking_levels.as_deref())
                .map_err(|e| e.to_string())?;
        let map = self.thinking_level_map.clone().unwrap_or_default();
        crate::model::validate_thinking_level_map(&map, &levels).map_err(|e| e.to_string())?;
        Ok((levels, map))
    }

    pub fn resolved_input_modalities(&self) -> Result<Vec<XyModality>, String> {
        let raw = self
            .input_modalities
            .clone()
            .unwrap_or_else(|| default_input_modalities(self.provider));
        if raw.is_empty() {
            return Err("input_modalities must not be empty".into());
        }
        let mut out = Vec::with_capacity(raw.len());
        for modality in raw {
            if !out.contains(&modality) {
                out.push(modality);
            }
        }
        if !out.contains(&XyModality::Text) {
            return Err("input_modalities must include text".into());
        }
        Ok(out)
    }

    pub fn to_model_meta(&self, id: &str) -> Result<crate::model::XyModelMeta, String> {
        use crate::model::{XyModelMeta, default_context_window_for};

        self.resolved_input_modalities()?;
        let config = self.to_model_config();
        let (thinking_levels, thinking_level_map) = self.resolve_thinking_config()?;
        let context_window = if self.context_window > 0 {
            self.context_window
        } else {
            default_context_window_for(config.kind)
        };
        let max_tokens = resolve_max_tokens(self.max_output_tokens)?;
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
            max_tokens,
            thinking_levels,
            thinking_level_map,
        })
    }

    pub fn to_ephemeral_meta(&self) -> Result<crate::model::XyModelMeta, String> {
        self.to_model_meta(&self.model)
    }
}

fn resolve_max_tokens(max_output_tokens: Option<u32>) -> Result<u64, String> {
    match max_output_tokens {
        Some(0) => Err("max_output_tokens must be greater than 0".into()),
        Some(value) => Ok(u64::from(value)),
        None => Ok(0),
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
    fn max_output_tokens_flows_into_meta() {
        let entry = XyModelEntryConfig {
            provider: XyModelKind::OpenAi,
            model: "m".into(),
            max_output_tokens: Some(4096),
            ..Default::default()
        };
        assert_eq!(entry.to_model_meta("m").expect("meta").max_tokens, 4096);
        let absent = XyModelEntryConfig {
            provider: XyModelKind::OpenAi,
            model: "m".into(),
            ..Default::default()
        };
        assert_eq!(absent.to_model_meta("m").expect("meta").max_tokens, 0);
    }

    #[test]
    fn explicit_context_window_overrides_default() {
        let entry = XyModelEntryConfig {
            provider: XyModelKind::OpenAi,
            model: "m".into(),
            context_window: 64_000,
            ..Default::default()
        };
        assert_eq!(
            entry.to_model_meta("m").expect("meta").context_window,
            64_000
        );
    }

    #[test]
    fn max_output_tokens_zero_is_rejected() {
        let entry = XyModelEntryConfig {
            provider: XyModelKind::OpenAi,
            model: "m".into(),
            max_output_tokens: Some(0),
            ..Default::default()
        };
        assert!(entry.to_model_meta("m").is_err());
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

    #[test]
    fn resolves_configured_input_modalities() {
        let yaml = r"
provider: openai
model: vision
input_modalities: [text, image, image]
";
        let entry: XyModelEntryConfig = yaml_serde::from_str(yaml).expect("yaml");
        assert_eq!(
            entry.resolved_input_modalities().expect("modalities"),
            vec![XyModality::Text, XyModality::Image]
        );
    }

    #[test]
    fn defaults_input_modalities_by_provider() {
        let openai = XyModelEntryConfig {
            provider: XyModelKind::OpenAi,
            ..Default::default()
        };
        assert_eq!(
            openai.resolved_input_modalities().expect("openai"),
            vec![XyModality::Text, XyModality::Image]
        );
        let fake = XyModelEntryConfig {
            provider: XyModelKind::Fake,
            ..Default::default()
        };
        assert_eq!(
            fake.resolved_input_modalities().expect("fake"),
            vec![XyModality::Text]
        );
    }

    #[test]
    fn rejects_empty_or_textless_modalities() {
        let empty = XyModelEntryConfig {
            provider: XyModelKind::OpenAi,
            input_modalities: Some(Vec::new()),
            ..Default::default()
        };
        assert!(empty.resolved_input_modalities().is_err());
        let textless = XyModelEntryConfig {
            provider: XyModelKind::OpenAi,
            input_modalities: Some(vec![XyModality::Image]),
            ..Default::default()
        };
        assert!(textless.resolved_input_modalities().is_err());
    }
}
