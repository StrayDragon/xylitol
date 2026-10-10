//! Context token estimation with provenance: Api, else Heuristic.

use crate::dto::{
    AiBridgeMessage, AiBridgeStopReason, AiBridgeUsage, ContextTokenEstimate, TokenProvenance,
};
use crate::usage::total_context_tokens;

/// Shared heuristic divisor: UTF-8 bytes, rounded up (r1929).
pub const HEURISTIC_BYTES_PER_TOKEN: u64 = 3;

/// Fold a UTF-8 byte length into heuristic tokens.
pub fn heuristic_token_count(byte_len: u64) -> u64 {
    byte_len.div_ceil(HEURISTIC_BYTES_PER_TOKEN)
}

#[derive(Default)]
pub struct EstimateContextOpts<'a> {
    pub last_usage: Option<&'a AiBridgeUsage>,
    pub stop_reason: Option<AiBridgeStopReason>,
}

fn is_valid_api_anchor(stop_reason: Option<AiBridgeStopReason>) -> bool {
    !matches!(
        stop_reason,
        Some(AiBridgeStopReason::Error | AiBridgeStopReason::Aborted)
    )
}

fn heuristic_tokens(messages: &[AiBridgeMessage]) -> u64 {
    messages
        .iter()
        .map(|msg| {
            let s = serde_json::to_string(msg).unwrap_or_default();
            heuristic_token_count(s.len() as u64)
        })
        .sum()
}

/// Index of the last assistant message that carries usage (pi `lastUsageIndex`).
fn last_assistant_usage_index(messages: &[AiBridgeMessage]) -> Option<usize> {
    messages
        .iter()
        .rposition(|m| matches!(m, AiBridgeMessage::AssistantMessage { usage: Some(_), .. }))
}

/// Estimate context tokens with priority: Api → Heuristic.
///
/// When an Api usage anchor is valid, trailing heuristic tokens cover only messages
/// **after** the last usage-bearing assistant (pi `estimateContextTokens`). If that
/// assistant is not present in `messages`, the whole slice is treated as trailing.
pub fn estimate_context(
    messages: &[AiBridgeMessage],
    opts: EstimateContextOpts<'_>,
) -> ContextTokenEstimate {
    if let Some(usage) = opts.last_usage
        && is_valid_api_anchor(opts.stop_reason)
    {
        let usage_tokens = total_context_tokens(usage);
        let last_usage_index = last_assistant_usage_index(messages);
        let trailing_tokens = match last_usage_index {
            Some(i) => heuristic_tokens(&messages[i + 1..]),
            None => heuristic_tokens(messages),
        };
        return ContextTokenEstimate {
            tokens: usage_tokens + trailing_tokens,
            provenance: TokenProvenance::Api,
            usage_tokens,
            trailing_tokens,
            last_usage_index,
        };
    }

    let tokens = heuristic_tokens(messages);
    ContextTokenEstimate {
        tokens,
        provenance: TokenProvenance::Heuristic,
        usage_tokens: 0,
        trailing_tokens: tokens,
        last_usage_index: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefer_api_anchor_when_valid() {
        let usage = AiBridgeUsage {
            input: 1000,
            output: 200,
            total_tokens: 1200,
            ..Default::default()
        };
        // Usage-bearing assistant not in slice → whole slice is trailing (pi-aligned).
        let msgs = vec![AiBridgeMessage::user("trailing")];
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: Some(&usage),
                stop_reason: Some(AiBridgeStopReason::Stop),
            },
        );
        assert_eq!(est.provenance, TokenProvenance::Api);
        assert_eq!(est.usage_tokens, 1200);
        assert!(est.trailing_tokens > 0);
        assert_eq!(est.tokens, est.usage_tokens + est.trailing_tokens);
        assert_eq!(est.last_usage_index, None);
    }

    #[test]
    fn api_trailing_only_after_usage_message() {
        let usage = AiBridgeUsage {
            input: 1000,
            output: 200,
            total_tokens: 1200,
            ..Default::default()
        };
        let history = AiBridgeMessage::user("x".repeat(8_000));
        let hist_alone = heuristic_tokens(std::slice::from_ref(&history));
        assert!(hist_alone > 500, "precondition: fat history heuristic");

        let assistant = AiBridgeMessage::AssistantMessage {
            content: vec![],
            stop_reason: Some(AiBridgeStopReason::Stop),
            usage: Some(usage),
            api: "test".into(),
            provider: "test".into(),
            model: "test".into(),
            response_id: None,
            error_message: None,
            timestamp: 1,
            diagnostics: Vec::new(),
        };
        let trail = AiBridgeMessage::user("after");
        let msgs = vec![history, assistant, trail.clone()];
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: Some(&usage),
                stop_reason: Some(AiBridgeStopReason::Stop),
            },
        );
        assert_eq!(est.provenance, TokenProvenance::Api);
        assert_eq!(est.usage_tokens, 1200);
        assert_eq!(est.last_usage_index, Some(1));
        let only_trail = heuristic_tokens(std::slice::from_ref(&trail));
        assert_eq!(est.trailing_tokens, only_trail);
        assert_eq!(est.tokens, 1200 + only_trail);
        // Must not double-count the fat history already covered by Api usage.
        assert!(est.tokens < 1200 + hist_alone);
    }

    #[test]
    fn abort_usage_must_not_be_api_anchor() {
        let usage = AiBridgeUsage {
            input: 500,
            output: 0,
            total_tokens: 500,
            ..Default::default()
        };
        let msgs = vec![AiBridgeMessage::user("hello")];
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: Some(&usage),
                stop_reason: Some(AiBridgeStopReason::Aborted),
            },
        );
        assert_ne!(est.provenance, TokenProvenance::Api);
        assert_eq!(est.provenance, TokenProvenance::Heuristic);
    }

    #[test]
    fn fallback_heuristic_without_anchor() {
        let msgs = vec![AiBridgeMessage::user("hello world")];
        let est = estimate_context(&msgs, EstimateContextOpts::default());
        assert_eq!(est.provenance, TokenProvenance::Heuristic);
        assert!(est.tokens > 0);
    }

    #[test]
    fn heuristic_divisor_is_three() {
        assert_eq!(heuristic_token_count(0), 0);
        assert_eq!(heuristic_token_count(1), 1);
        assert_eq!(heuristic_token_count(3), 1);
        assert_eq!(heuristic_token_count(4), 2);
    }
}
