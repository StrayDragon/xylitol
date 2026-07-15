//! Multi-source context token estimation with provenance.

use crate::dto::{
    AiBridgeMessage, AiBridgeStopReason, AiBridgeUsage, ContextTokenEstimate, TokenProvenance,
};
use crate::usage::total_context_tokens;

pub type RemoteCountFn = dyn Fn(&[AiBridgeMessage]) -> Option<u64> + Send + Sync;
pub type TokenizerEstimateFn = dyn Fn(&[AiBridgeMessage]) -> u64 + Send + Sync;

#[derive(Default)]
pub struct EstimateContextOpts<'a> {
    pub last_usage: Option<&'a AiBridgeUsage>,
    pub stop_reason: Option<AiBridgeStopReason>,
    pub remote_count: Option<Box<RemoteCountFn>>,
    pub tokenizer_estimate: Option<Box<TokenizerEstimateFn>>,
    pub allow_remote: bool,
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
            (s.len() as u64).div_ceil(4)
        })
        .sum()
}

/// Estimate context tokens with priority: Api → RemoteCount → LocalTokenizer → Heuristic.
pub fn estimate_context(
    messages: &[AiBridgeMessage],
    opts: EstimateContextOpts<'_>,
) -> ContextTokenEstimate {
    if let Some(usage) = opts.last_usage
        && is_valid_api_anchor(opts.stop_reason)
    {
        let usage_tokens = total_context_tokens(usage);
        let trailing_tokens = heuristic_tokens(messages);
        return ContextTokenEstimate {
            tokens: usage_tokens + trailing_tokens,
            provenance: TokenProvenance::Api,
            usage_tokens,
            trailing_tokens,
            last_usage_index: Some(0),
        };
    }

    if opts.allow_remote
        && let Some(remote) = &opts.remote_count
        && let Some(tokens) = remote(messages)
    {
        return ContextTokenEstimate {
            tokens,
            provenance: TokenProvenance::RemoteCount,
            usage_tokens: 0,
            trailing_tokens: tokens,
            last_usage_index: None,
        };
    }

    if let Some(tokenizer) = &opts.tokenizer_estimate {
        let tokens = tokenizer(messages);
        return ContextTokenEstimate {
            tokens,
            provenance: TokenProvenance::LocalTokenizer,
            usage_tokens: 0,
            trailing_tokens: tokens,
            last_usage_index: None,
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
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: 0,
            total_tokens: 1200,
            cost: None,
        };
        let msgs = vec![AiBridgeMessage::user("trailing")];
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: Some(&usage),
                stop_reason: Some(AiBridgeStopReason::Stop),
                ..Default::default()
            },
        );
        assert_eq!(est.provenance, TokenProvenance::Api);
        assert_eq!(est.usage_tokens, 1200);
        assert!(est.trailing_tokens > 0);
        assert_eq!(est.tokens, est.usage_tokens + est.trailing_tokens);
    }

    #[test]
    fn abort_usage_must_not_be_api_anchor() {
        let usage = AiBridgeUsage {
            input: 500,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: 0,
            total_tokens: 500,
            cost: None,
        };
        let msgs = vec![AiBridgeMessage::user("hello")];
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: Some(&usage),
                stop_reason: Some(AiBridgeStopReason::Aborted),
                ..Default::default()
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
}
