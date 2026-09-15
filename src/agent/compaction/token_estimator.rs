//! Token estimation — multi-source accounting via xylitol-ai-bridge (c1030 / c1210).

use xylitol_ai_bridge::accounting::{EstimateContextOpts, estimate_context};
use xylitol_ai_bridge::dto::AiBridgeMessage;
use xylitol_ai_bridge::registry::{TokenizerSource, resolve_tokenizer_with_override};
use xylitol_ai_bridge::tokenize::HfTokenizerCache;
use xylitol_ai_bridge::tokenize::{BuiltinTokenizer, estimate_messages};

use crate::agent::llm_project::project_for_llm;
use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage, XyStopReason, XyUsage};
use crate::protocol::model::{ContextTokenEstimate, XyToolSchema};

/// Fixed per-request context outside the session transcript (c25 / c16).
///
/// System prompt + tool schemas ride every provider call but live outside the
/// session entries; estimates MUST fold them in on non-Api provenance so the
/// footer / reserve gate see the real next-request size.
#[derive(Debug, Clone, Default)]
pub struct FixedRequestContext {
    pub system_prompt: Option<String>,
    pub tool_schemas: Vec<XyToolSchema>,
}

impl FixedRequestContext {
    /// chars/4 estimate of the fixed overhead (same unit as the cut walk, c8).
    pub fn overhead_tokens(&self) -> u64 {
        let mut chars: u64 = self
            .system_prompt
            .as_deref()
            .map(str::len)
            .unwrap_or_default() as u64;
        for t in &self.tool_schemas {
            chars += (t.name.len() + t.description.len()) as u64;
            chars += t.parameters.to_string().len() as u64;
        }
        chars.div_ceil(4)
    }

    /// Estimate-only pseudo rows folding the fixed context into message accounting.
    fn pseudo_rows(&self) -> Vec<AiBridgeMessage> {
        fn user_row(text: String) -> AiBridgeMessage {
            LlmMessage::UserMessage {
                content: vec![AgentPart::text(text)],
                timestamp: 0,
            }
        }
        let mut rows = Vec::new();
        if let Some(sp) = self.system_prompt.as_deref().filter(|s| !s.is_empty()) {
            rows.push(user_row(sp.to_string()));
        }
        if !self.tool_schemas.is_empty() {
            let blob = self
                .tool_schemas
                .iter()
                .map(|t| format!("- {} ({}): {}", t.name, t.description, t.parameters))
                .collect::<Vec<_>>()
                .join("\n");
            rows.push(user_row(format!(
                "[Fixed request context: tool schemas]\n{blob}"
            )));
        }
        rows
    }
}

/// Options for [`estimate_context_tokens_with`].
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
    /// Fixed per-request overhead folded in when no Api anchor exists (c25).
    /// Api provenance MUST NOT receive this — usage.input already covers the
    /// full request; folding it in would double-count.
    pub fixed_context: Option<FixedRequestContext>,
    /// When true, emit a `token.estimate` fastrace span (c1860). Default **false** —
    /// settlement paths call `emit_token_estimate_obs` explicitly so callers do not
    /// each create duplicate OTel observations.
    pub emit_obs: bool,
    /// Optional `agent.turn` parent for `token.estimate` nesting.
    pub obs_parent: Option<fastrace::prelude::SpanContext>,
    /// Obs session snapshot for the `token.estimate` span (otel24 / c2610);
    /// empty default stamps no session attributes (quiet estimates never emit).
    pub obs_session: xylitol_ai_bridge::ObsSessionContext,
}

/// Build a [`ContextTokenEstimate`] from persisted session entries (footer + compact).
pub fn estimate_from_session_entries(
    entries: &[crate::protocol::session::SessionEntry],
    opts: &EstimateOpts,
) -> ContextTokenEstimate {
    use crate::protocol::session::{SessionEntry, build_context_entries};

    let entries = build_context_entries(entries);
    let mut messages: Vec<AgentMessage> = Vec::new();
    let mut last_usage: Option<XyUsage> = None;
    let mut stop_reason = None;
    let mut usage_anchor_ms: u64 = 0;
    let mut latest_compaction_ms: u64 = 0;

    for entry in &entries {
        if let SessionEntry::Compaction(c) = entry {
            latest_compaction_ms = latest_compaction_ms.max(c.base.timestamp);
        }
        if let SessionEntry::Message(m) = entry
            && let Ok(msg) = serde_json::from_value::<AgentMessage>(m.message.clone())
        {
            if let AgentMessage::Llm(LlmMessage::AssistantMessage {
                usage: Some(u),
                stop_reason: sr,
                timestamp,
                ..
            }) = &msg
            {
                last_usage = Some(*u);
                stop_reason = *sr;
                usage_anchor_ms = if *timestamp > 0 {
                    *timestamp
                } else {
                    m.base.timestamp
                };
            }
            messages.push(msg);
        }
    }

    // c25: a usage anchor not newer than the latest compaction describes the
    // pre-compact request — drop it so the post-cut estimate (placeholder)
    // falls back to per-message accounting instead of overstating.
    if usage_anchor_ms > 0 && usage_anchor_ms <= latest_compaction_ms {
        last_usage = None;
        stop_reason = None;
    }

    estimate_context_tokens_with(&messages, last_usage.as_ref(), stop_reason, opts)
}

/// Estimate context tokens via accounting priority:
/// Api → RemoteCount → LocalTokenizer → Heuristic. Full entry with optional stop-reason
/// (Api anchor validity) and model id.
pub fn estimate_context_tokens_with(
    messages: &[AgentMessage],
    last_usage: Option<&XyUsage>,
    stop_reason: Option<XyStopReason>,
    opts: &EstimateOpts,
) -> ContextTokenEstimate {
    // LlmMessage ≡ AiBridgeMessage (c1210); project_for_llm is the sole map.
    let mut bridge_msgs: Vec<AiBridgeMessage> = project_for_llm(messages);
    // c25: fold fixed request overhead (system prompt + tools) only when no Api
    // anchor carries it already.
    if last_usage.is_none()
        && let Some(fixed) = &opts.fixed_context
    {
        bridge_msgs.extend(fixed.pseudo_rows());
    }
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

    let result: ContextTokenEstimate = est;
    log::debug!(
        target: "xylitol::token_estimate",
        "token estimate backend={} tokens={} usage_tokens={} trailing={} allow_local={} allow_remote={} emit_obs={} model_id={:?}",
        result.provenance.as_str(),
        result.tokens,
        result.usage_tokens,
        result.trailing_tokens,
        opts.allow_local_tokenizer,
        opts.allow_remote_count,
        opts.emit_obs,
        opts.model_id,
    );
    if opts.emit_obs {
        emit_token_estimate_obs(&result, opts);
    }
    result
}

/// Record which estimate backend won (fastrace), gated like provider-trace.
///
/// Prefer [`super::settlement::settle_from_session_entries`] so compact + footer
/// share one emit; do not call this from every estimate caller.
pub(crate) fn emit_token_estimate_obs(est: &ContextTokenEstimate, opts: &EstimateOpts) {
    let backend = est.provenance.as_str();
    if !xylitol_ai_bridge::provider::trace::provider_trace_active() {
        return;
    }
    use fastrace::prelude::*;
    let mut props = vec![
        ("provenance".into(), backend.to_string()),
        ("tokens".into(), est.tokens.to_string()),
        ("usage_tokens".into(), est.usage_tokens.to_string()),
        ("trailing_tokens".into(), est.trailing_tokens.to_string()),
    ];
    // Gate flags: only emit when armed (default false is noise).
    if opts.allow_local_tokenizer {
        props.push(("allow_local_tokenizer".into(), "true".into()));
    }
    if opts.allow_remote_count {
        props.push(("allow_remote_count".into(), "true".into()));
    }
    if let Some(model) = opts
        .model_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        props.push(("model_id".into(), model.to_string()));
    }
    props.extend(xylitol_ai_bridge::provider::langfuse_session_properties_from(&opts.obs_session));
    props.extend(xylitol_ai_bridge::provider::xylitol_obs_lane_properties(
        xylitol_ai_bridge::provider::XYLITOL_OBS_LANE_LLM,
    ));
    // Prefer explicit agent.turn parent; otherwise independent root (same session attrs).
    let parent = opts.obs_parent.unwrap_or_else(SpanContext::random);
    let span = Span::root("token.estimate", parent).with_properties(|| props);
    span.add_event(Event::new("token.estimate").with_properties(|| {
        [
            ("kind", "token.estimate".to_string()),
            ("provenance", backend.to_string()),
            ("tokens", est.tokens.to_string()),
        ]
    }));
    // Drop span → flush via reporter.
    drop(span);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::message::AgentMessage;
    use crate::protocol::model::TokenProvenance;
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

    fn fixed_fixture() -> FixedRequestContext {
        FixedRequestContext {
            system_prompt: Some("S".repeat(4_000)),
            tool_schemas: vec![crate::protocol::model::XyToolSchema {
                name: "read".into(),
                description: "d".repeat(200),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            }],
        }
    }

    /// c25/c16: overhead folds into Heuristic estimates but never double-counts
    /// when an Api anchor already covers the full request.
    #[test]
    fn fixed_context_folds_into_non_api_only() {
        let msgs = [AgentMessage::user("hello world")];
        let fixed = fixed_fixture();

        let base = estimate_context_tokens_with(&msgs, None, None, &EstimateOpts::default());
        let with_over = estimate_context_tokens_with(
            &msgs,
            None,
            None,
            &EstimateOpts {
                fixed_context: Some(fixed.clone()),
                ..Default::default()
            },
        );
        assert_eq!(base.provenance, TokenProvenance::Heuristic);
        assert!(
            with_over.tokens > base.tokens + fixed.overhead_tokens() - 50,
            "overhead must land in the estimate: base={} with={}",
            base.tokens,
            with_over.tokens
        );

        let usage = XyUsage {
            input: 5_000,
            output: 0,
            total_tokens: 5_000,
            ..Default::default()
        };
        let api_plain = estimate_context_tokens_with(
            &msgs,
            Some(&usage),
            Some(XyStopReason::Stop),
            &EstimateOpts::default(),
        );
        let api_over = estimate_context_tokens_with(
            &msgs,
            Some(&usage),
            Some(XyStopReason::Stop),
            &EstimateOpts {
                fixed_context: Some(fixed),
                ..Default::default()
            },
        );
        assert_eq!(
            api_plain.tokens, api_over.tokens,
            "Api anchor already covers system+tools; MUST NOT double-count"
        );
    }

    /// c25: after a compaction, the only surviving usage anchor describes the
    /// pre-compact request; the AfterCompaction placeholder MUST fall back to
    /// per-message accounting instead of reusing the stale full-request number.
    #[test]
    fn usage_anchor_not_newer_than_compaction_is_dropped() {
        use crate::protocol::message::{LlmMessage, XyStopReason, XyUsage};
        use crate::protocol::model::TokenProvenance;
        use crate::protocol::session::{CompactionEntry, EntryBase, MessageEntry, SessionEntry};

        let usage = XyUsage {
            input: 90_000,
            output: 0,
            total_tokens: 90_000,
            ..Default::default()
        };
        let kept = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![crate::protocol::message::AgentPart::text("kept tail")],
            stop_reason: Some(XyStopReason::Stop),
            usage: Some(usage),
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 100,
            diagnostics: Vec::new(),
        });
        let msg_entry = |id: &str, parent: Option<&str>, msg: &AgentMessage, ts: u64| {
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: id.into(),
                    parent_id: parent.map(str::to_string),
                    timestamp: ts,
                },
                message: serde_json::to_value(msg).unwrap(),
            })
        };
        let compaction = SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: Some("a_kept".into()),
                timestamp: 200,
            },
            summary: "prior turns summarized".into(),
            first_kept_entry_id: "a_kept".into(),
            tokens_before: 90_000,
            details: None,
            from_hook: None,
        });
        let entries = vec![
            msg_entry("a_kept", None, &kept, 100),
            compaction.clone(),
            msg_entry("u_new", Some("c1"), &AgentMessage::user("continue"), 201),
        ];

        let est = estimate_from_session_entries(&entries, &EstimateOpts::default());
        assert_eq!(
            est.provenance,
            TokenProvenance::Heuristic,
            "pre-compact Api anchor MUST be dropped for the placeholder"
        );
        assert!(
            est.tokens < 1_000,
            "placeholder reflects summary+tail, not the stale 90k: {}",
            est.tokens
        );

        // A fresh anchor (timestamp after the compaction) still wins.
        let fresh = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![crate::protocol::message::AgentPart::text("fresh")],
            stop_reason: Some(XyStopReason::Stop),
            usage: Some(usage),
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 300,
            diagnostics: Vec::new(),
        });
        let with_fresh = estimate_from_session_entries(
            &[
                msg_entry("a_kept", None, &kept, 100),
                compaction.clone(),
                msg_entry("a_fresh", Some("c1"), &fresh, 300),
            ],
            &EstimateOpts::default(),
        );
        assert_eq!(with_fresh.provenance, TokenProvenance::Api);
    }

    #[test]
    fn session_entries_api_usage_anchors_estimate() {
        use crate::protocol::message::{LlmMessage, XyStopReason, XyUsage};
        use crate::protocol::model::TokenProvenance;
        use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};

        let usage = XyUsage {
            input: 100,
            output: 20,
            total_tokens: 120,
            ..Default::default()
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
                    timestamp: 0,
                },
                message: serde_json::to_value(AgentMessage::user("hi")).unwrap(),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: serde_json::to_value(asst).unwrap(),
            }),
        ];
        let est = estimate_from_session_entries(&entries, &EstimateOpts::default());
        assert_eq!(est.provenance, TokenProvenance::Api);
        assert!(est.tokens > 0);
        // Reserve formula must use this shared number (not an independent len/4 sum).
        let settings = crate::agent::compaction::CompactionSettings::default();
        let _ = crate::agent::compaction::should_compact(est.tokens, 128_000, &settings, 0);
    }
}
