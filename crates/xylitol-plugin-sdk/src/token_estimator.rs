//! Token estimation — multi-source accounting via xylitol-official-plugin-provider-openai (c1030 / c1210).

use crate::accounting::{EstimateContextOpts, estimate_context, heuristic_token_count};
use crate::dto::AiBridgeMessage;

use crate::message::{AgentMessage, AgentPart, LlmMessage, XyStopReason, XyUsage};
use crate::model::{ContextTokenEstimate, XyToolSchema};
use crate::project_outbound;

/// Stable identifier for the estimate algorithm recorded in compaction history.
pub const ESTIMATOR_VERSION: &str = "context-estimate-v1";

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
    /// `/3` estimate of the fixed overhead (same unit as the cut walk, c8).
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
        heuristic_token_count(chars)
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
    pub obs_session: crate::ObsSessionContext,
}

/// Build a [`ContextTokenEstimate`] from persisted session entries (footer + compact).
pub fn estimate_from_session_entries(
    entries: &[crate::session::SessionEntry],
    opts: &EstimateOpts,
) -> ContextTokenEstimate {
    use crate::session::{SessionEntry, build_context_entries};

    let bar = crate::AgentStatusBar::from_session_entries(entries);
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

    estimate_context_tokens_with(&messages, last_usage.as_ref(), stop_reason, opts, &bar)
}

/// Estimate context tokens via accounting priority: Api → Heuristic.
pub fn estimate_context_tokens_with(
    messages: &[AgentMessage],
    last_usage: Option<&XyUsage>,
    stop_reason: Option<XyStopReason>,
    opts: &EstimateOpts,
    bar: &crate::AgentStatusBar,
) -> ContextTokenEstimate {
    // History fold + status bar (r1124). Compact summarizer does not
    // call this path; it uses `project_for_llm` only.
    let mut bridge_msgs: Vec<AiBridgeMessage> = project_outbound(messages, bar);
    // c25: fold fixed request overhead (system prompt + tools) only when no Api
    // anchor carries it already.
    if last_usage.is_none()
        && let Some(fixed) = &opts.fixed_context
    {
        bridge_msgs.extend(fixed.pseudo_rows());
    }
    let bridge_usage = last_usage.copied();
    let bridge_stop = stop_reason;

    let est = estimate_context(
        &bridge_msgs,
        EstimateContextOpts {
            last_usage: bridge_usage.as_ref(),
            stop_reason: bridge_stop,
        },
    );

    let result: ContextTokenEstimate = est;
    log::debug!(
        target: "xylitol::token_estimate",
        "token estimate backend={} tokens={} usage_tokens={} trailing={} emit_obs={} model_id={:?}",
        result.provenance.as_str(),
        result.tokens,
        result.usage_tokens,
        result.trailing_tokens,
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
pub fn emit_token_estimate_obs(est: &ContextTokenEstimate, opts: &EstimateOpts) {
    let backend = est.provenance.as_str();
    if !crate::provider_trace_active() {
        return;
    }
    use fastrace::prelude::*;
    let mut props = vec![
        ("provenance".into(), backend.to_string()),
        ("tokens".into(), est.tokens.to_string()),
        ("usage_tokens".into(), est.usage_tokens.to_string()),
        ("trailing_tokens".into(), est.trailing_tokens.to_string()),
    ];
    if let Some(model) = opts
        .model_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        props.push(("model_id".into(), model.to_string()));
    }
    props.extend(crate::langfuse_session_properties_from(&opts.obs_session));
    props.extend(crate::xylitol_obs_lane_properties(
        crate::XYLITOL_OBS_LANE_LLM,
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
