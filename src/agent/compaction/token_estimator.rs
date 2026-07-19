//! Token estimation — multi-source accounting via xylitol-ai-bridge (c1030).

use xylitol_ai_bridge::accounting::{EstimateContextOpts, estimate_context};
use xylitol_ai_bridge::dto::{
    AiBridgeImageContent, AiBridgeMessage, AiBridgePart, AiBridgeStopReason, AiBridgeUsage,
    AiBridgeUsageCost, Diagnostic as AiBridgeDiagnostic, TokenProvenance as AiBridgeProvenance,
};
use xylitol_ai_bridge::registry::{TokenizerSource, resolve_tokenizer_with_override};
use xylitol_ai_bridge::tokenize::HfTokenizerCache;
use xylitol_ai_bridge::tokenize::{BuiltinTokenizer, estimate_messages};

use crate::domain::llm_project::project_for_llm;
use crate::domain::message::{
    AgentMessage, AgentPart, Diagnostic, ImageContent, LlmMessage, XyStopReason, XyUsage,
};
use crate::domain::types::{ContextTokenEstimate, TokenProvenance};

/// Calculate total context tokens from a XyUsage struct.
/// Priority: total_tokens > input+output+cache_read+cache_write sum.
pub fn calculate_context_tokens(usage: &XyUsage) -> u64 {
    if usage.total_tokens > 0 {
        return usage.total_tokens;
    }
    usage.input + usage.output + usage.cache_read + usage.cache_write
}

/// Backward-compatible alias used by older compaction tests.
pub type ContextUsageEstimate = ContextTokenEstimate;

fn to_bridge_llm_message(msg: LlmMessage) -> AiBridgeMessage {
    match msg {
        LlmMessage::UserMessage { content, timestamp } => AiBridgeMessage::UserMessage {
            content: content.into_iter().map(to_bridge_part).collect(),
            timestamp,
        },
        LlmMessage::AssistantMessage {
            content,
            stop_reason,
            usage,
            api,
            provider,
            model,
            response_id,
            error_message,
            timestamp,
            diagnostics,
        } => AiBridgeMessage::AssistantMessage {
            content: content.into_iter().map(to_bridge_part).collect(),
            stop_reason: stop_reason.map(to_bridge_stop),
            usage: usage.as_ref().map(to_bridge_usage),
            api,
            provider,
            model,
            response_id,
            error_message,
            timestamp,
            diagnostics: diagnostics.into_iter().map(to_bridge_diagnostic).collect(),
        },
        LlmMessage::ToolResultMessage {
            tool_use_id,
            tool_name,
            content,
            details,
            is_error,
            timestamp,
        } => AiBridgeMessage::ToolResultMessage {
            tool_use_id,
            tool_name,
            content: content.into_iter().map(to_bridge_part).collect(),
            details,
            is_error,
            timestamp,
        },
    }
}

fn to_bridge_part(part: AgentPart) -> AiBridgePart {
    match part {
        AgentPart::Text { text } => AiBridgePart::Text { text },
        AgentPart::Image(img) => AiBridgePart::Image(to_bridge_image(img)),
        AgentPart::Thinking {
            thinking,
            redacted,
            thinking_signature,
        } => AiBridgePart::Thinking {
            thinking,
            redacted,
            thinking_signature,
        },
        AgentPart::ToolCall {
            id,
            name,
            arguments,
        } => AiBridgePart::ToolCall {
            id,
            name,
            arguments,
        },
    }
}

fn to_bridge_image(img: ImageContent) -> AiBridgeImageContent {
    AiBridgeImageContent {
        url: img.url,
        data: img.data,
        media_type: img.media_type,
    }
}

fn to_bridge_diagnostic(d: Diagnostic) -> AiBridgeDiagnostic {
    AiBridgeDiagnostic {
        message: d.message,
        source: d.source,
    }
}

fn to_bridge_usage(u: &XyUsage) -> AiBridgeUsage {
    AiBridgeUsage {
        input: u.input,
        output: u.output,
        cache_read: u.cache_read,
        cache_write: u.cache_write,
        cache_write_1h: u.cache_write_1h,
        total_tokens: u.total_tokens,
        cost: u.cost.map(|c| AiBridgeUsageCost {
            input: c.input,
            output: c.output,
            cache_read: c.cache_read,
            cache_write: c.cache_write,
            total: c.total,
        }),
    }
}

fn to_bridge_stop(r: XyStopReason) -> AiBridgeStopReason {
    match r {
        XyStopReason::Stop => AiBridgeStopReason::Stop,
        XyStopReason::MaxTokens => AiBridgeStopReason::MaxTokens,
        XyStopReason::Error => AiBridgeStopReason::Error,
        XyStopReason::Aborted => AiBridgeStopReason::Aborted,
        XyStopReason::ToolUse => AiBridgeStopReason::ToolUse,
    }
}

fn to_domain_provenance(p: AiBridgeProvenance) -> TokenProvenance {
    match p {
        AiBridgeProvenance::Api => TokenProvenance::Api,
        AiBridgeProvenance::RemoteCount => TokenProvenance::RemoteCount,
        AiBridgeProvenance::LocalTokenizer => TokenProvenance::LocalTokenizer,
        AiBridgeProvenance::Heuristic => TokenProvenance::Heuristic,
        AiBridgeProvenance::Unknown => TokenProvenance::Unknown,
    }
}

/// Options for [`estimate_context_tokens`].
#[derive(Debug, Clone, Default)]
pub struct EstimateOpts {
    pub model_id: Option<String>,
    /// Config / CLI-derived override (c1380 `ModelEntry.tokenizer`).
    pub tokenizer_override: Option<xylitol_ai_bridge::registry::TokenizerOverride>,
    /// When false (default), RemoteCount is skipped (must be explicitly enabled).
    pub allow_remote_count: bool,
    /// Injected RemoteCount result (tests / Anthropic count_tokens caller).
    pub remote_count_tokens: Option<u64>,
}

/// Estimate context tokens via accounting priority:
/// Api → RemoteCount → LocalTokenizer → Heuristic.
pub fn estimate_context_tokens(
    messages: &[AgentMessage],
    last_usage: Option<&XyUsage>,
) -> ContextTokenEstimate {
    estimate_context_tokens_with(messages, last_usage, None, &EstimateOpts::default())
}

/// Full estimate entry with optional stop-reason (Api anchor validity) and model id.
pub fn estimate_context_tokens_with(
    messages: &[AgentMessage],
    last_usage: Option<&XyUsage>,
    stop_reason: Option<XyStopReason>,
    opts: &EstimateOpts,
) -> ContextTokenEstimate {
    let bridge_msgs: Vec<AiBridgeMessage> = project_for_llm(messages)
        .into_iter()
        .map(to_bridge_llm_message)
        .collect();
    let bridge_usage = last_usage.map(to_bridge_usage);
    let bridge_stop = stop_reason.map(to_bridge_stop);

    let model_id = opts.model_id.clone();
    let tok_over = opts.tokenizer_override.clone();
    let remote_tokens = opts.remote_count_tokens;
    let allow_remote = opts.allow_remote_count;

    let tokenizer_estimate: Option<Box<xylitol_ai_bridge::accounting::TokenizerEstimateFn>> =
        model_id.as_ref().and_then(|id| {
            let source = resolve_tokenizer_with_override(id, tok_over.clone())?;
            Some(Box::new(move |msgs: &[AiBridgeMessage]| match &source {
                TokenizerSource::Builtin(b) => estimate_messages(msgs, *b),
                TokenizerSource::HuggingFace { repo, file } => {
                    let cache = HfTokenizerCache::new(None);
                    msgs.iter()
                        .map(|m| {
                            let s = serde_json::to_string(m).unwrap_or_default();
                            cache
                                .encode_count_if_cached(repo, file, &s)
                                .unwrap_or_else(|| BuiltinTokenizer::OpenAiCl100k.encode_count(&s))
                        })
                        .sum()
                }
                TokenizerSource::Local { path } => {
                    let cache = HfTokenizerCache::new(None);
                    msgs.iter()
                        .map(|m| {
                            let s = serde_json::to_string(m).unwrap_or_default();
                            cache
                                .encode_count_at_path(path, &s)
                                .unwrap_or_else(|| BuiltinTokenizer::OpenAiCl100k.encode_count(&s))
                        })
                        .sum()
                }
            })
                as Box<xylitol_ai_bridge::accounting::TokenizerEstimateFn>)
        });

    let remote_count: Option<Box<xylitol_ai_bridge::accounting::RemoteCountFn>> = if allow_remote {
        Some(Box::new(move |_msgs: &[AiBridgeMessage]| remote_tokens))
    } else {
        None
    };

    let est = estimate_context(
        &bridge_msgs,
        EstimateContextOpts {
            last_usage: bridge_usage.as_ref(),
            stop_reason: bridge_stop,
            remote_count,
            tokenizer_estimate,
            allow_remote,
        },
    );

    ContextTokenEstimate {
        tokens: est.tokens,
        provenance: to_domain_provenance(est.provenance),
        usage_tokens: est.usage_tokens,
        trailing_tokens: est.trailing_tokens,
        last_usage_index: est.last_usage_index,
    }
}

/// Check if compaction should trigger based on a token reserve threshold.
#[cfg(test)]
pub(crate) fn should_compact_by_reserve(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::AgentMessage;
    use crate::domain::types::TokenProvenance;
    use xylitol_ai_bridge::registry::TokenizerOverride;

    #[test]
    fn override_enables_local_tokenizer_for_unmapped_alias() {
        let msgs = [AgentMessage::user("hello world")];
        let without_over = estimate_context_tokens_with(
            &msgs,
            None,
            None,
            &EstimateOpts {
                model_id: Some("qwen-custom".into()),
                ..Default::default()
            },
        );
        assert_eq!(without_over.provenance, TokenProvenance::Heuristic);

        let with_over = estimate_context_tokens_with(
            &msgs,
            None,
            None,
            &EstimateOpts {
                model_id: Some("qwen-custom".into()),
                tokenizer_override: Some(TokenizerOverride::Builtin),
                ..Default::default()
            },
        );
        assert_eq!(with_over.provenance, TokenProvenance::LocalTokenizer);
        assert!(with_over.tokens > 0);
    }
}
