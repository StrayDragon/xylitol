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
