//! Session statistics and context-usage estimation (spec c255 / as32).

use crate::protocol::error::XyError;
use crate::protocol::ports::XySessionStore;

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
pub async fn compute(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<SessionStats, XyError> {
    let ctx = store
        .build_session_context(session_id)
        .await
        .map_err(XyError::from)?;
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
pub fn estimate_tokens(messages: &[crate::protocol::message::AgentMessage]) -> u64 {
    crate::agent::compaction::token_estimator::estimate_context_tokens(messages, None).tokens
}

/// Compute context usage info from a token estimate and window size.
///
/// `percent` is a derived display value only; trigger uses reserve formula via `settings`.
pub fn get_context_usage(
    token_estimate: u64,
    context_window: u64,
    settings: &crate::agent::compaction::CompactionSettings,
) -> ContextUsage {
    let percent = if context_window > 0 {
        ((token_estimate as f64 / context_window as f64) * 100.0) as u64
    } else {
        0
    };
    ContextUsage {
        tokens: token_estimate,
        context_window,
        percent,
        should_compact: should_compact(token_estimate, context_window, settings),
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::agent::compaction::{CompactionSettings, should_compact};
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(512))]

        /// Context usage is monotone in token estimate: adding a message can
        /// never reduce percent or un-trigger compaction.
        #[test]
        fn usage_monotone_in_tokens(
            tokens in 0u64..500_000,
            window in 1u64..200_000,
            reserve in 0u64..50_000,
            enabled in any::<bool>(),
        ) {
            let settings = CompactionSettings {
                enabled,
                reserve_tokens: reserve,
                keep_recent_tokens: 0,
            };
            let a = get_context_usage(tokens, window, &settings);
            let b = get_context_usage(tokens.saturating_add(1), window, &settings);
            prop_assert!(b.tokens >= a.tokens);
            prop_assert!(b.percent >= a.percent);
            prop_assert_eq!(
                b.should_compact,
                should_compact(tokens + 1, window, &settings)
            );
            // Compaction state may flip on but never off as tokens grow.
            if a.should_compact {
                prop_assert!(b.should_compact);
            }
        }
    }
}
