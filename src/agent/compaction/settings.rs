//! Compaction settings — master toggle, reserved tokens, keep-recent threshold.

/// Settings controlling compaction behavior.
#[derive(Debug, Clone)]
pub struct CompactionSettings {
    /// Master toggle.
    pub enabled: bool,
    /// Tokens reserved for the summarization LLM call itself.
    pub reserve_tokens: u64,
    /// Target number of tokens to keep in the recent context window.
    pub keep_recent_tokens: u64,
}

impl Default for CompactionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        }
    }
}

impl From<crate::domain::compaction_config::XyCompactionSettingsConfig> for CompactionSettings {
    fn from(s: crate::domain::compaction_config::XyCompactionSettingsConfig) -> Self {
        Self {
            enabled: s.enabled.unwrap_or(true),
            reserve_tokens: s.reserve_tokens.unwrap_or(16384),
            keep_recent_tokens: s.keep_recent_tokens.unwrap_or(20000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_settings_uses_defaults_when_none() {
        let s: CompactionSettings =
            crate::domain::compaction_config::XyCompactionSettingsConfig::default().into();
        assert!(s.enabled);
        assert_eq!(s.reserve_tokens, 16384);
        assert_eq!(s.keep_recent_tokens, 20000);
    }

    #[test]
    fn from_settings_applies_overrides() {
        let src = crate::domain::compaction_config::XyCompactionSettingsConfig {
            enabled: Some(false),
            reserve_tokens: Some(1000),
            keep_recent_tokens: Some(5000),
        };
        let s: CompactionSettings = src.into();
        assert!(!s.enabled);
        assert_eq!(s.reserve_tokens, 1000);
        assert_eq!(s.keep_recent_tokens, 5000);
    }
}
