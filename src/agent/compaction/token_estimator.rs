//! Token estimation — heuristic token counting for compaction decisions.

/// Provider usage information from an assistant message.
#[derive(Debug, Clone, Copy)]
pub struct XyUsage {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

/// Calculate total context tokens from a Usage struct.
/// Priority: total_tokens > input+output+cache_read+cache_write sum.
pub fn calculate_context_tokens(usage: &XyUsage) -> u64 {
    if usage.total_tokens > 0 {
        return usage.total_tokens;
    }
    usage.input_tokens + usage.output_tokens + usage.cache_read_tokens + usage.cache_write_tokens
}

/// Structure returned by [`estimate_context_tokens`].
#[derive(Debug, Clone)]
pub struct ContextUsageEstimate {
    pub tokens: u64,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

/// Estimate context tokens from messages.
/// Uses chars/4 heuristic per message.
/// When a `last_usage` is provided, uses real usage tokens and
/// estimates only trailing messages.
pub fn estimate_context_tokens(
    messages: &[crate::core::message::AgentMessage],
    last_usage: Option<&XyUsage>,
) -> ContextUsageEstimate {
    if let Some(usage) = last_usage {
        let usage_tokens = calculate_context_tokens(usage);
        let mut trailing_tokens = 0u64;
        for msg in messages {
            trailing_tokens += estimate_tokens_agent(msg);
        }
        ContextUsageEstimate {
            tokens: usage_tokens + trailing_tokens,
            usage_tokens,
            trailing_tokens,
            last_usage_index: Some(0),
        }
    } else {
        let mut total = 0u64;
        for msg in messages {
            total += estimate_tokens_agent(msg);
        }
        ContextUsageEstimate {
            tokens: total,
            usage_tokens: 0,
            trailing_tokens: total,
            last_usage_index: None,
        }
    }
}

/// Estimate tokens for an AgentMessage using chars/4.
fn estimate_tokens_agent(msg: &crate::core::message::AgentMessage) -> u64 {
    let s = serde_json::to_string(msg).unwrap_or_default();
    (s.len() as u64).div_ceil(4)
}

/// Check if compaction should trigger based on context token usage.
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: &super::settings::CompactionSettings,
) -> bool {
    if !settings.enabled {
        return false;
    }
    let threshold = context_window.saturating_sub(settings.reserve_tokens);
    context_tokens > threshold
}
