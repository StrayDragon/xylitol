//! Compaction settings — master toggle, reserved tokens, keep-recent threshold.

use xylitol_plugin_sdk::model_entry::XyModelEntryConfig;

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

impl From<xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig>
    for CompactionSettings
{
    fn from(s: xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig) -> Self {
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
    config: Option<&xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig>,
    settings: Option<&xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig>,
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
