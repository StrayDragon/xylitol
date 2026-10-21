use crate::model_entry::XyModelEntryConfig;

#[derive(Debug, Clone)]
pub struct CompactionSettings {
    pub enabled: bool,
    pub reserve_tokens: u64,
    pub keep_recent_tokens: u64,
    pub model: Option<XyModelEntryConfig>,
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

impl From<crate::compaction_config::XyCompactionSettingsConfig> for CompactionSettings {
    fn from(s: crate::compaction_config::XyCompactionSettingsConfig) -> Self {
        Self {
            enabled: s.enabled.unwrap_or(true),
            reserve_tokens: s.reserve_tokens.unwrap_or(16384),
            keep_recent_tokens: s.keep_recent_tokens.unwrap_or(20000),
            model: s.model,
            thinking_level: s.thinking_level,
        }
    }
}

pub fn merge_compaction_runtime(
    config: Option<&crate::compaction_config::XyCompactionSettingsConfig>,
    settings: Option<&crate::compaction_config::XyCompactionSettingsConfig>,
) -> CompactionSettings {
    let base = settings
        .map(|s| CompactionSettings {
            enabled: s.enabled.unwrap_or(true),
            reserve_tokens: s.reserve_tokens.unwrap_or(16384),
            keep_recent_tokens: s.keep_recent_tokens.unwrap_or(20000),
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

pub fn effective_trigger_threshold(
    context_window: u64,
    settings: &CompactionSettings,
    post_compact_floor: u64,
) -> u64 {
    context_window
        .saturating_sub(settings.reserve_tokens)
        .max(post_compact_floor + post_compact_floor / 4)
}
