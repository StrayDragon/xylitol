//! Session statistics and context-usage estimation (spec c255 / as32).

/// Statistics for a session.
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub total_messages: usize,
    pub thinking_level: String,
    pub model: Option<(String, String)>,
}

/// Context usage summary for compaction decisions.
#[derive(Debug, Clone)]
pub struct ContextUsage {
    pub tokens: u64,
    pub context_window: u64,
    pub percent: u64,
    pub should_compact: bool,
}

pub use crate::agent::compaction_orchestrator::should_compact;

/// Estimate token count from messages using simple heuristic (1 token ≈ 4 chars).
pub fn estimate_tokens(messages: &[crate::core::message::AgentMessage]) -> u64 {
    let mut total = 0u64;
    for msg in messages {
        for part in msg.content() {
            match part {
                crate::core::message::AgentPart::Text(s)
                | crate::core::message::AgentPart::Thinking { text: s, .. } => {
                    total += (s.len() as u64).div_ceil(4);
                }
                crate::core::message::AgentPart::ToolCall {
                    name, arguments, ..
                } => {
                    total += (name.len() as u64).div_ceil(4);
                    total += (arguments.to_string().len() as u64).div_ceil(4);
                }
                crate::core::message::AgentPart::ToolResult { content, .. } => {
                    for inner in content {
                        if let crate::core::message::AgentPart::Text(s) = inner {
                            total += (s.len() as u64).div_ceil(4);
                        }
                    }
                }
                crate::core::message::AgentPart::Image(_) => {
                    total += 4800; // image token estimate
                }
            }
        }
    }
    total
}

/// Compute context usage info from a token estimate and window size.
pub fn get_context_usage(token_estimate: u64, context_window: u64, threshold: f64) -> ContextUsage {
    let percent = if context_window > 0 {
        ((token_estimate as f64 / context_window as f64) * 100.0) as u64
    } else {
        0
    };
    ContextUsage {
        tokens: token_estimate,
        context_window,
        percent,
        should_compact: should_compact(token_estimate, context_window, threshold),
    }
}
