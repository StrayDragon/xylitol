//! Token estimation — multi-source accounting via xylitol-ai-bridge (c1030 / c1210).

use xylitol_ai_bridge::accounting::{EstimateContextOpts, estimate_context};
use xylitol_ai_bridge::dto::{AiBridgeMessage, TokenProvenance as AiBridgeProvenance};
use xylitol_ai_bridge::registry::{TokenizerSource, resolve_tokenizer_with_override};
use xylitol_ai_bridge::tokenize::HfTokenizerCache;
use xylitol_ai_bridge::tokenize::{BuiltinTokenizer, estimate_messages};

use crate::agent::llm_project::project_for_llm;
use crate::protocol::message::{AgentMessage, LlmMessage, XyStopReason, XyUsage};
use crate::protocol::types::{ContextTokenEstimate, TokenProvenance};

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
    /// When false (default, c1420 / paa10), LocalTokenizer encode is skipped.
    pub allow_local_tokenizer: bool,
}

/// Build a [`ContextTokenEstimate`] from persisted session entries (footer + compact).
pub fn estimate_from_session_entries(
    entries: &[crate::protocol::session::SessionEntry],
    opts: &EstimateOpts,
) -> ContextTokenEstimate {
    use crate::protocol::session::SessionEntry;

    let mut messages: Vec<AgentMessage> = Vec::new();
    let mut last_usage: Option<XyUsage> = None;
    let mut stop_reason = None;

    for entry in entries {
        if let SessionEntry::Message(m) = entry
            && let Ok(msg) = serde_json::from_value::<AgentMessage>(m.message.clone())
        {
            if let AgentMessage::Llm(LlmMessage::AssistantMessage {
                usage: Some(u),
                stop_reason: sr,
                ..
            }) = &msg
            {
                last_usage = Some(*u);
                stop_reason = *sr;
            }
            messages.push(msg);
        }
    }

    estimate_context_tokens_with(&messages, last_usage.as_ref(), stop_reason, opts)
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
    // LlmMessage ≡ AiBridgeMessage (c1210); project_for_llm is the sole map.
    let bridge_msgs: Vec<AiBridgeMessage> = project_for_llm(messages);
    let bridge_usage = last_usage.copied();
    let bridge_stop = stop_reason;

    let model_id = opts.model_id.clone();
    let tok_over = opts.tokenizer_override.clone();
    let remote_tokens = opts.remote_count_tokens;
    let allow_remote = opts.allow_remote_count;

    let tokenizer_estimate: Option<Box<xylitol_ai_bridge::accounting::TokenizerEstimateFn>> =
        if opts.allow_local_tokenizer {
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
                                    .unwrap_or_else(|| {
                                        BuiltinTokenizer::OpenAiCl100k.encode_count(&s)
                                    })
                            })
                            .sum()
                    }
                    TokenizerSource::Local { path } => {
                        let cache = HfTokenizerCache::new(None);
                        msgs.iter()
                            .map(|m| {
                                let s = serde_json::to_string(m).unwrap_or_default();
                                cache.encode_count_at_path(path, &s).unwrap_or_else(|| {
                                    BuiltinTokenizer::OpenAiCl100k.encode_count(&s)
                                })
                            })
                            .sum()
                    }
                })
                    as Box<xylitol_ai_bridge::accounting::TokenizerEstimateFn>)
            })
        } else {
            None
        };

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

    let result = ContextTokenEstimate {
        tokens: est.tokens,
        provenance: to_domain_provenance(est.provenance),
        usage_tokens: est.usage_tokens,
        trailing_tokens: est.trailing_tokens,
        last_usage_index: est.last_usage_index,
    };
    emit_token_estimate_obs(&result, opts);
    result
}

/// Record which estimate backend won (fastrace + level log), gated like provider-trace.
fn emit_token_estimate_obs(est: &ContextTokenEstimate, opts: &EstimateOpts) {
    let backend = est.provenance.as_str();
    log::debug!(
        target: "xylitol::token_estimate",
        "token estimate backend={backend} tokens={} usage_tokens={} trailing={} allow_local={} allow_remote={} model_id={:?}",
        est.tokens,
        est.usage_tokens,
        est.trailing_tokens,
        opts.allow_local_tokenizer,
        opts.allow_remote_count,
        opts.model_id,
    );

    if !xylitol_ai_bridge::provider::trace::provider_trace_active() {
        return;
    }
    use fastrace::prelude::*;
    let model = opts.model_id.clone().unwrap_or_default();
    let mut props = vec![
        ("backend".into(), backend.to_string()),
        ("provenance".into(), backend.to_string()),
        ("tokens".into(), est.tokens.to_string()),
        ("usage_tokens".into(), est.usage_tokens.to_string()),
        ("trailing_tokens".into(), est.trailing_tokens.to_string()),
        (
            "allow_local_tokenizer".into(),
            opts.allow_local_tokenizer.to_string(),
        ),
        (
            "allow_remote_count".into(),
            opts.allow_remote_count.to_string(),
        ),
        ("model_id".into(), model),
    ];
    props.extend(xylitol_ai_bridge::provider::langfuse_session_properties());
    let span = Span::root("token.estimate", SpanContext::random()).with_properties(|| props);
    span.add_event(Event::new("token.estimate").with_properties(|| {
        [
            ("kind", "token.estimate".to_string()),
            ("backend", backend.to_string()),
            ("tokens", est.tokens.to_string()),
        ]
    }));
    // Drop span → flush via reporter.
    drop(span);
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
    use crate::protocol::message::AgentMessage;
    use crate::protocol::types::TokenProvenance;
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
                allow_local_tokenizer: true,
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
                allow_local_tokenizer: true,
                ..Default::default()
            },
        );
        assert_eq!(with_over.provenance, TokenProvenance::LocalTokenizer);
        assert!(with_over.tokens > 0);
    }

    #[test]
    fn local_tokenizer_off_skips_encode_even_with_override() {
        let msgs = [AgentMessage::user("hello world")];
        let est = estimate_context_tokens_with(
            &msgs,
            None,
            None,
            &EstimateOpts {
                model_id: Some("qwen-custom".into()),
                tokenizer_override: Some(TokenizerOverride::Builtin),
                allow_local_tokenizer: false,
                ..Default::default()
            },
        );
        assert_ne!(est.provenance, TokenProvenance::LocalTokenizer);
        assert_eq!(est.provenance, TokenProvenance::Heuristic);
    }

    #[test]
    fn session_entries_api_usage_anchors_estimate() {
        use crate::protocol::message::{LlmMessage, XyStopReason, XyUsage};
        use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};
        use crate::protocol::types::TokenProvenance;

        let usage = XyUsage {
            input: 100,
            output: 20,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: 0,
            total_tokens: 120,
            cost: None,
        };
        let asst = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![crate::protocol::message::AgentPart::text("ok")],
            stop_reason: Some(XyStopReason::Stop),
            usage: Some(usage),
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        });
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: String::new(),
                },
                message: serde_json::to_value(AgentMessage::user("hi")).unwrap(),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: None,
                    timestamp: String::new(),
                },
                message: serde_json::to_value(asst).unwrap(),
            }),
        ];
        let est = estimate_from_session_entries(&entries, &EstimateOpts::default());
        assert_eq!(est.provenance, TokenProvenance::Api);
        assert!(est.tokens > 0);
        // Threshold must use this shared number (not an independent len/4 sum).
        let _ = crate::agent::compaction::should_compact(est.tokens, 128_000, 0.8);
    }
}
