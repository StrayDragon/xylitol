use crate::dto::{ContextTokenEstimate, TokenProvenance, XyLlmMessage, XyStopReason, XyUsage};

pub const HEURISTIC_BYTES_PER_TOKEN: u64 = 3;

pub fn heuristic_token_count(byte_len: u64) -> u64 {
    byte_len.div_ceil(HEURISTIC_BYTES_PER_TOKEN)
}

pub fn total_context_tokens(usage: &XyUsage) -> u64 {
    if usage.total_tokens > 0 {
        return usage.total_tokens;
    }
    usage.input + usage.output + usage.cache_read + usage.cache_write
}

#[derive(Default)]
pub struct EstimateContextOpts<'a> {
    pub last_usage: Option<&'a XyUsage>,
    pub stop_reason: Option<XyStopReason>,
}

fn is_valid_api_anchor(stop_reason: Option<XyStopReason>) -> bool {
    !matches!(
        stop_reason,
        Some(XyStopReason::Error | XyStopReason::Aborted)
    )
}

fn heuristic_tokens(messages: &[XyLlmMessage]) -> u64 {
    messages
        .iter()
        .map(|msg| {
            let s = serde_json::to_string(msg).unwrap_or_default();
            heuristic_token_count(s.len() as u64)
        })
        .sum()
}

fn last_assistant_usage_index(messages: &[XyLlmMessage]) -> Option<usize> {
    messages
        .iter()
        .rposition(|m| matches!(m, XyLlmMessage::AssistantMessage { usage: Some(_), .. }))
}

pub fn estimate_context(
    messages: &[XyLlmMessage],
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
        let usage = XyUsage {
            input: 1000,
            output: 200,
            total_tokens: 1200,
            ..Default::default()
        };
        let msgs = vec![XyLlmMessage::user("trailing")];
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: Some(&usage),
                stop_reason: Some(XyStopReason::Stop),
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
        let usage = XyUsage {
            input: 1000,
            output: 200,
            total_tokens: 1200,
            ..Default::default()
        };
        let history = XyLlmMessage::user("x".repeat(8_000));
        let hist_alone = heuristic_tokens(std::slice::from_ref(&history));
        assert!(hist_alone > 500, "precondition: fat history heuristic");

        let assistant = XyLlmMessage::AssistantMessage {
            content: vec![],
            stop_reason: Some(XyStopReason::Stop),
            usage: Some(usage),
            api: "test".into(),
            provider: "test".into(),
            model: "test".into(),
            response_id: None,
            error_message: None,
            timestamp: 1,
            diagnostics: Vec::new(),
        };
        let trail = XyLlmMessage::user("after");
        let msgs = vec![history, assistant, trail.clone()];
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: Some(&usage),
                stop_reason: Some(XyStopReason::Stop),
            },
        );
        assert_eq!(est.provenance, TokenProvenance::Api);
        assert_eq!(est.usage_tokens, 1200);
        assert_eq!(est.last_usage_index, Some(1));
        let only_trail = heuristic_tokens(std::slice::from_ref(&trail));
        assert_eq!(est.trailing_tokens, only_trail);
        assert_eq!(est.tokens, 1200 + only_trail);
        assert!(est.tokens < 1200 + hist_alone);
    }

    #[test]
    fn abort_usage_must_not_be_api_anchor() {
        let usage = XyUsage {
            input: 500,
            output: 0,
            total_tokens: 500,
            ..Default::default()
        };
        let msgs = vec![XyLlmMessage::user("hello")];
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: Some(&usage),
                stop_reason: Some(XyStopReason::Aborted),
            },
        );
        assert_ne!(est.provenance, TokenProvenance::Api);
        assert_eq!(est.provenance, TokenProvenance::Heuristic);
    }

    #[test]
    fn fallback_heuristic_without_anchor() {
        let msgs = vec![XyLlmMessage::user("hello world")];
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
