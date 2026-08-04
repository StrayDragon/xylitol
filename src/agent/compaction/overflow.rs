//! Context-overflow detection — pi `isContextOverflow` aligned.

use regex::Regex;
use std::sync::LazyLock;

use crate::protocol::message::{AgentMessage, LlmMessage, XyStopReason};

static OVERFLOW_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)prompt is too long",
        r"(?i)request_too_large",
        r"(?i)input is too long for requested model",
        r"(?i)exceeds the context window",
        r"(?i)exceeds (?:the )?(?:model'?s )?maximum context length(?: of [\d,]+ tokens?|\s*\([\d,]+\))",
        r"(?i)input token count.*exceeds the maximum",
        r"(?i)maximum prompt length is \d+",
        r"(?i)reduce the length of the messages",
        r"(?i)maximum context length is \d+ tokens",
        r"(?i)exceeds (?:the )?maximum allowed input length of [\d,]+ tokens?",
        r"(?i)input \(\d+ tokens\) is longer than the model'?s context length \(\d+ tokens\)",
        r"(?i)exceeds the limit of \d+",
        r"(?i)exceeds the available context size",
        r"(?i)greater than the context length",
        r"(?i)context window exceeds limit",
        r"(?i)exceeded model token limit",
        r"(?i)too large for model with \d+ maximum context length",
        r"(?i)prompt has [\d,]+ tokens?, but the configured context size is [\d,]+ tokens?",
        r"(?i)model_context_window_exceeded",
        r"(?i)prompt too long; exceeded (?:max )?context length",
        r"(?i)range of input length should be",
        r"(?i)context[_ ]length[_ ]exceeded",
        r"(?i)too many tokens",
        r"(?i)token limit exceeded",
        r"(?i)^4(?:00|13)\s*(?:status code)?\s*\(no body\)",
        r"(?i)Input length too long",
        r"(?i)exceeds \d+ token limit",
    ]
    .into_iter()
    .map(|p| Regex::new(p).expect("overflow pattern"))
    .collect()
});

static NON_OVERFLOW_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)^(Throttling error|Service unavailable):",
        r"(?i)rate limit",
        r"(?i)too many requests",
    ]
    .into_iter()
    .map(|p| Regex::new(p).expect("non-overflow pattern"))
    .collect()
});

/// Whether `error_message` matches overflow patterns (and not non-overflow exclusions).
pub fn error_message_is_context_overflow(error_message: &str) -> bool {
    if NON_OVERFLOW_PATTERNS
        .iter()
        .any(|p| p.is_match(error_message))
    {
        return false;
    }
    OVERFLOW_PATTERNS.iter().any(|p| p.is_match(error_message))
}

/// pi `isContextOverflow(assistant, contextWindow)`.
pub fn is_context_overflow_assistant(message: &AgentMessage, context_window: u64) -> bool {
    let AgentMessage::Llm(LlmMessage::AssistantMessage {
        stop_reason,
        error_message,
        usage,
        ..
    }) = message
    else {
        return false;
    };

    // Case 1: error message patterns
    if *stop_reason == Some(XyStopReason::Error)
        && let Some(msg) = error_message.as_deref()
        && error_message_is_context_overflow(msg)
    {
        return true;
    }

    if context_window == 0 {
        return false;
    }

    let Some(u) = usage else {
        return false;
    };
    let input_tokens = u.input + u.cache_read;

    // Case 2: silent overflow (successful but usage exceeds window)
    if *stop_reason == Some(XyStopReason::Stop) && input_tokens > context_window {
        return true;
    }

    // Case 3: length-stop with zero output filling the window
    if *stop_reason == Some(XyStopReason::MaxTokens)
        && u.output == 0
        && input_tokens >= (context_window as f64 * 0.99) as u64
    {
        return true;
    }

    false
}

/// Whether assistant provider+model matches the current model (pi `sameModel`).
pub fn assistant_same_model(message: &AgentMessage, provider: &str, model_id: &str) -> bool {
    let AgentMessage::Llm(LlmMessage::AssistantMessage {
        provider: p,
        model: m,
        ..
    }) = message
    else {
        return false;
    };
    // Empty tags on a just-produced turn: treat as same (legacy / partial fill).
    if p.is_empty() && m.is_empty() {
        return true;
    }
    p == provider && (m == model_id || model_id.ends_with(m.as_str()) || m.ends_with(model_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::message::{AgentPart, XyUsage};

    fn asst(stop: XyStopReason, err: Option<&str>, usage: Option<XyUsage>) -> AgentMessage {
        AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::text("x")],
            stop_reason: Some(stop),
            usage,
            api: String::new(),
            provider: "fake".into(),
            model: "m1".into(),
            response_id: None,
            error_message: err.map(str::to_string),
            timestamp: 1,
            diagnostics: Vec::new(),
        })
    }

    #[test]
    fn detects_anthropic_style_error() {
        let m = asst(
            XyStopReason::Error,
            Some("prompt is too long: 213462 tokens > 200000 maximum"),
            None,
        );
        assert!(is_context_overflow_assistant(&m, 200_000));
    }

    #[test]
    fn excludes_rate_limit() {
        let m = asst(
            XyStopReason::Error,
            Some("rate limit: too many tokens, please wait"),
            None,
        );
        assert!(!is_context_overflow_assistant(&m, 200_000));
    }

    #[test]
    fn silent_usage_over_window() {
        let m = asst(
            XyStopReason::Stop,
            None,
            Some(XyUsage {
                input: 210_000,
                output: 10,
                total_tokens: 210_010,
                ..Default::default()
            }),
        );
        assert!(is_context_overflow_assistant(&m, 200_000));
        assert!(!is_context_overflow_assistant(&m, 300_000));
    }

    #[test]
    fn length_zero_output_fills_window() {
        let m = asst(
            XyStopReason::MaxTokens,
            None,
            Some(XyUsage {
                input: 199_000,
                output: 0,
                total_tokens: 199_000,
                ..Default::default()
            }),
        );
        assert!(is_context_overflow_assistant(&m, 200_000));
    }

    #[test]
    fn same_model_empty_ok() {
        let mut m = asst(XyStopReason::Error, Some("prompt is too long"), None);
        if let AgentMessage::Llm(LlmMessage::AssistantMessage {
            provider, model, ..
        }) = &mut m
        {
            provider.clear();
            model.clear();
        }
        assert!(assistant_same_model(&m, "openai", "gpt"));
    }

    #[test]
    fn wrong_model_rejected() {
        let m = asst(XyStopReason::Error, Some("prompt is too long"), None);
        assert!(!assistant_same_model(&m, "openai", "gpt-4o"));
        assert!(assistant_same_model(&m, "fake", "m1"));
    }
}
