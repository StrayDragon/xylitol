use crate::accounting::{EstimateContextOpts, estimate_context, heuristic_token_count};
use crate::message::{AgentMessage, AgentPart, XyLlmMessage, XyStopReason, XyUsage};
use crate::model::{ContextTokenEstimate, XyToolSchema};
use crate::project_outbound;

pub const ESTIMATOR_VERSION: &str = "context-estimate-v1";

#[derive(Debug, Clone, Default)]
pub struct FixedRequestContext {
    pub system_prompt: Option<String>,
    pub tool_schemas: Vec<XyToolSchema>,
}

impl FixedRequestContext {
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

    fn pseudo_rows(&self) -> Vec<XyLlmMessage> {
        fn user_row(text: String) -> XyLlmMessage {
            XyLlmMessage::UserMessage {
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

#[derive(Debug, Clone, Default)]
pub struct EstimateOpts {
    pub model_id: Option<String>,
    pub fixed_context: Option<FixedRequestContext>,
    pub emit_obs: bool,
    pub obs_parent: Option<fastrace::prelude::SpanContext>,
    pub obs_session: crate::ObsSessionContext,
}

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
            if let AgentMessage::Llm(XyLlmMessage::AssistantMessage {
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

    if usage_anchor_ms > 0 && usage_anchor_ms <= latest_compaction_ms {
        last_usage = None;
        stop_reason = None;
    }

    estimate_context_tokens_with(&messages, last_usage.as_ref(), stop_reason, opts, &bar)
}

pub fn estimate_context_tokens_with(
    messages: &[AgentMessage],
    last_usage: Option<&XyUsage>,
    stop_reason: Option<XyStopReason>,
    opts: &EstimateOpts,
    bar: &crate::AgentStatusBar,
) -> ContextTokenEstimate {
    let mut bridge_msgs: Vec<XyLlmMessage> = project_outbound(messages, bar);
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
    let parent = opts.obs_parent.unwrap_or_else(SpanContext::random);
    let span = Span::root("token.estimate", parent).with_properties(|| props);
    span.add_event(Event::new("token.estimate").with_properties(|| {
        [
            ("kind", "token.estimate".to_string()),
            ("provenance", backend.to_string()),
            ("tokens", est.tokens.to_string()),
        ]
    }));
    drop(span);
}
