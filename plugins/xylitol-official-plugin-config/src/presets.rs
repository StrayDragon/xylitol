use serde_json::{Map, Value, json};

use super::error::LoadError;
use xylitol_plugin_sdk::model::XyModelKind;

pub struct ProviderPreset {
    pub id: &'static str,
    pub provider: XyModelKind,
    pub api: Option<&'static str>,
    pub compat: Option<&'static str>,
    pub base_url: &'static str,
    pub default_model: Option<&'static str>,
    pub thinking: bool,
    pub thinking_levels: Option<&'static [&'static str]>,
    pub context_window: u64,
}

pub const PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "zmodel",
        provider: XyModelKind::OpenAi,
        api: Some("openai-completions"),
        compat: None,
        base_url: "https://api.z.ai/api/paas/v4",
        default_model: Some("glm-5.3"),
        thinking: true,
        thinking_levels: Some(&["low", "high", "max"]),
        context_window: 1_000_000,
    },
    ProviderPreset {
        id: "opencode-zen",
        provider: XyModelKind::OpenAi,
        api: Some("openai-completions"),
        compat: Some("deepseek"),
        base_url: "https://opencode.ai/zen/v1",
        default_model: Some("deepseek-v4-flash-free"),
        thinking: true,
        thinking_levels: Some(&["off", "low", "high", "max"]),
        context_window: 200_000,
    },
    ProviderPreset {
        id: "clinepass",
        provider: XyModelKind::OpenAi,
        api: Some("openai-completions"),
        compat: None,
        base_url: "https://api.cline.bot/api/v1",
        default_model: Some("cline-pass/deepseek-v4.1-flash"),
        thinking: true,
        thinking_levels: Some(&["off", "low", "high", "max"]),
        context_window: 0,
    },
    ProviderPreset {
        id: "commandcode",
        provider: XyModelKind::OpenAi,
        api: Some("openai-completions"),
        compat: None,
        base_url: "https://api.commandcode.ai/provider/v1",
        default_model: Some("deepseek/deepseek-v4-pro"),
        thinking: true,
        thinking_levels: Some(&["off", "low", "high", "max"]),
        context_window: 0,
    },
];

pub fn find(id: &str) -> Option<&'static ProviderPreset> {
    PRESETS.iter().find(|preset| preset.id == id)
}

pub fn apply_presets(merged: &mut Value) -> Result<(), LoadError> {
    let Some(models) = merged
        .get_mut("models")
        .and_then(|models| models.get_mut("models"))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };

    for (name, entry) in models.iter_mut() {
        let Some(entry) = entry.as_object_mut() else {
            continue;
        };
        let Some(preset_id) = entry.get("preset").and_then(Value::as_str) else {
            continue;
        };
        let preset = find(preset_id).ok_or_else(|| {
            LoadError::validation(format!(
                "unknown preset: {preset_id} (models.{name}); known presets: {}",
                PRESETS
                    .iter()
                    .map(|preset| preset.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
        fill_defaults(entry, preset);
    }
    Ok(())
}

fn fill_defaults(entry: &mut Map<String, Value>, preset: &ProviderPreset) {
    entry
        .entry("provider")
        .or_insert_with(|| json!(preset.provider.provider_name()));
    if let Some(model) = preset.default_model {
        entry.entry("model").or_insert_with(|| json!(model));
    }
    entry
        .entry("base_url")
        .or_insert_with(|| json!(preset.base_url));
    if let Some(api) = preset.api {
        entry.entry("api").or_insert_with(|| json!(api));
    }
    if let Some(compat) = preset.compat {
        entry.entry("compat").or_insert_with(|| json!(compat));
    }
    entry
        .entry("thinking")
        .or_insert_with(|| json!(preset.thinking));
    if let Some(levels) = preset.thinking_levels {
        entry
            .entry("thinking_levels")
            .or_insert_with(|| json!(levels));
    }
    if preset.context_window > 0 {
        entry
            .entry("context_window")
            .or_insert_with(|| json!(preset.context_window));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_are_unique_and_documented() {
        for preset in PRESETS {
            assert!(!preset.id.is_empty());
            assert_eq!(PRESETS.iter().filter(|p| p.id == preset.id).count(), 1);
            assert!(!preset.base_url.is_empty());
        }
    }

    #[test]
    fn preset_fills_connection_defaults_without_secrets() {
        let mut merged = json!({
            "models": {
                "models": {
                    "glm": { "preset": "zmodel" }
                }
            }
        });
        apply_presets(&mut merged).expect("preset");
        let entry = &merged["models"]["models"]["glm"];
        assert_eq!(entry["provider"], "openai");
        assert_eq!(entry["model"], "glm-5.3");
        assert_eq!(entry["base_url"], "https://api.z.ai/api/paas/v4");
        assert_eq!(entry["api"], "openai-completions");
        assert_eq!(entry["thinking"], true);
        assert_eq!(entry["thinking_levels"], json!(["low", "high", "max"]));
        assert_eq!(entry["context_window"], 1_000_000);
        assert!(
            entry.get("api_key").is_none(),
            "presets must not carry keys"
        );
    }

    #[test]
    fn explicit_fields_override_preset_defaults() {
        let mut merged = json!({
            "models": {
                "models": {
                    "glm": {
                        "preset": "zmodel",
                        "model": "glm-5.2",
                        "base_url": "https://example.test/v1",
                        "thinking": false,
                        "thinking_levels": ["off"],
                        "context_window": 128000
                    }
                }
            }
        });
        apply_presets(&mut merged).expect("preset");
        let entry = &merged["models"]["models"]["glm"];
        assert_eq!(entry["model"], "glm-5.2");
        assert_eq!(entry["base_url"], "https://example.test/v1");
        assert_eq!(entry["thinking"], false);
        assert_eq!(entry["thinking_levels"], json!(["off"]));
        assert_eq!(entry["context_window"], 128000);
        assert_eq!(entry["api"], "openai-completions");
    }

    #[test]
    fn unknown_preset_is_a_readable_error() {
        let mut merged = json!({
            "models": {
                "models": {
                    "mystery": { "preset": "no-such-preset" }
                }
            }
        });
        let err = apply_presets(&mut merged).expect_err("unknown preset");
        let text = err.to_string();
        assert!(text.contains("unknown preset: no-such-preset"), "{text}");
        assert!(text.contains("models.mystery"), "{text}");
    }

    #[test]
    fn entries_without_preset_are_untouched() {
        let mut merged = json!({
            "models": {
                "models": {
                    "plain": { "provider": "anthropic", "model": "claude" }
                }
            }
        });
        apply_presets(&mut merged).expect("no preset");
        assert_eq!(merged["models"]["models"]["plain"]["provider"], "anthropic");
        assert_eq!(merged["models"]["models"]["plain"]["model"], "claude");
    }
}
