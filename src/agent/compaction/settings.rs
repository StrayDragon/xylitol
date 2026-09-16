//! Compaction settings — master toggle, reserved tokens, keep-recent threshold.

use crate::protocol::model_entry::XyModelEntryConfig;

/// Settings controlling compaction behavior.
#[derive(Debug, Clone)]
pub struct CompactionSettings {
    /// Master toggle.
    pub enabled: bool,
    /// Tokens reserved for the summarization LLM call itself.
    pub reserve_tokens: u64,
    /// Target number of tokens to keep in the recent context window.
    pub keep_recent_tokens: u64,
    /// Optional task-scoped summary model entry (config.yaml only).
    pub model: Option<XyModelEntryConfig>,
    /// Optional thinking-level override for summary requests (config.yaml only).
    pub thinking_level: Option<String>,
}

impl Default for CompactionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
            model: None,
            thinking_level: None,
        }
    }
}

impl From<crate::protocol::compaction_config::XyCompactionSettingsConfig> for CompactionSettings {
    fn from(s: crate::protocol::compaction_config::XyCompactionSettingsConfig) -> Self {
        Self {
            enabled: s.enabled.unwrap_or(true),
            reserve_tokens: s.reserve_tokens.unwrap_or(16384),
            keep_recent_tokens: s.keep_recent_tokens.unwrap_or(20000),
            model: s.model,
            thinking_level: s.thinking_level,
        }
    }
}

/// Merge settings.json compaction with config.yaml-only task fields (rc15).
pub fn merge_compaction_runtime(
    config: Option<&crate::protocol::compaction_config::XyCompactionSettingsConfig>,
    settings: Option<&crate::protocol::compaction_config::XyCompactionSettingsConfig>,
) -> CompactionSettings {
    let base = settings
        .map(|s| CompactionSettings {
            enabled: s.enabled.unwrap_or(true),
            reserve_tokens: s.reserve_tokens.unwrap_or(16384),
            keep_recent_tokens: s.keep_recent_tokens.unwrap_or(20000),
            // rc15: task model + thinking override are config.yaml-only.
            model: None,
            thinking_level: None,
        })
        .unwrap_or_default();
    let Some(cfg) = config else {
        return base;
    };
    CompactionSettings {
        enabled: cfg.enabled.or(Some(base.enabled)).unwrap_or(base.enabled),
        reserve_tokens: cfg.reserve_tokens.unwrap_or(base.reserve_tokens),
        keep_recent_tokens: cfg.keep_recent_tokens.unwrap_or(base.keep_recent_tokens),
        model: cfg.model.clone(),
        thinking_level: cfg.thinking_level.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::compaction_config::XyCompactionSettingsConfig;
    use crate::protocol::model::XyModelKind;

    #[test]
    fn from_settings_uses_defaults_when_none() {
        let s: CompactionSettings = XyCompactionSettingsConfig::default().into();
        assert!(s.enabled);
        assert_eq!(s.reserve_tokens, 16384);
        assert_eq!(s.keep_recent_tokens, 20000);
        assert!(s.model.is_none());
        assert!(s.thinking_level.is_none());
    }

    #[test]
    fn from_settings_applies_overrides() {
        let src = XyCompactionSettingsConfig {
            enabled: Some(false),
            reserve_tokens: Some(1000),
            keep_recent_tokens: Some(5000),
            model: Some(XyModelEntryConfig {
                provider: XyModelKind::Fake,
                model: "summary".into(),
                ..Default::default()
            }),
            thinking_level: Some("high".into()),
        };
        let s: CompactionSettings = src.into();
        assert!(!s.enabled);
        assert_eq!(s.reserve_tokens, 1000);
        assert_eq!(s.keep_recent_tokens, 5000);
        assert_eq!(s.model.as_ref().map(|m| m.model.as_str()), Some("summary"));
        assert_eq!(s.thinking_level.as_deref(), Some("high"));
    }

    #[test]
    fn merge_runtime_overlays_config_task_fields() {
        let settings = XyCompactionSettingsConfig {
            enabled: Some(true),
            reserve_tokens: Some(8000),
            keep_recent_tokens: Some(10000),
            model: None,
            thinking_level: None,
        };
        let config = XyCompactionSettingsConfig {
            enabled: None,
            reserve_tokens: None,
            keep_recent_tokens: None,
            model: Some(XyModelEntryConfig {
                provider: XyModelKind::Fake,
                model: "cheap".into(),
                ..Default::default()
            }),
            thinking_level: Some("max".into()),
        };
        let merged = merge_compaction_runtime(Some(&config), Some(&settings));
        assert_eq!(merged.reserve_tokens, 8000);
        assert_eq!(
            merged.model.as_ref().map(|m| m.model.as_str()),
            Some("cheap")
        );
        assert_eq!(merged.thinking_level.as_deref(), Some("max"));
    }

    #[test]
    fn merge_runtime_config_yaml_overrides_settings_scalars() {
        let settings = XyCompactionSettingsConfig {
            enabled: Some(true),
            reserve_tokens: Some(8000),
            keep_recent_tokens: Some(10_000),
            model: None,
            thinking_level: None,
        };
        let config = XyCompactionSettingsConfig {
            enabled: Some(false),
            reserve_tokens: Some(12_000),
            keep_recent_tokens: Some(42_000),
            model: None,
            thinking_level: None,
        };
        let merged = merge_compaction_runtime(Some(&config), Some(&settings));
        assert!(!merged.enabled);
        assert_eq!(merged.reserve_tokens, 12_000);
        assert_eq!(merged.keep_recent_tokens, 42_000);
    }
}
