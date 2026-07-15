//! Session statistics and context-usage estimation (spec c255 / as32).

use crate::runtime_protocol::XySessionStore;

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

/// Aggregate message counts for a session by reading the session store.
///
/// Moved out of the `AgentCapabilities` body (spec as32 / c320 T24) so the capability
/// aggregate holds no aggregation logic.
pub async fn compute(store: &dyn XySessionStore, session_id: &str) -> Result<SessionStats, String> {
    let ctx = store.build_session_context(session_id).await?;
    let user_messages = ctx
        .messages
        .iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .count();
    let assistant_messages = ctx
        .messages
        .iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"))
        .count();
    let total_messages = ctx.messages.len();

    Ok(SessionStats {
        session_id: session_id.to_string(),
        user_messages,
        assistant_messages,
        total_messages,
        thinking_level: ctx.thinking_level,
        model: ctx.model,
    })
}

/// Context usage summary for compaction decisions.
#[derive(Debug, Clone)]
pub struct ContextUsage {
    pub tokens: u64,
    pub context_window: u64,
    pub percent: u64,
    pub should_compact: bool,
}

pub use crate::agent::compaction::orchestrator::should_compact;

/// Estimate token count from messages via accounting (Heuristic fallback).
pub fn estimate_tokens(messages: &[crate::domain::message::AgentMessage]) -> u64 {
    crate::agent::compaction::token_estimator::estimate_context_tokens(messages, None).tokens
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
