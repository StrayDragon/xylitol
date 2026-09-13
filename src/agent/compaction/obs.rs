//! Fastrace / Langfuse span for session compaction (`agent.compaction`, c1700).
//!
//! Lives here (not `agent/runtime/obs`) so `CompactionOrchestrator` can emit without
//! a runtime ↔ compaction cycle. Gated like `token.estimate` / ReAct low-freq spans.

use fastrace::prelude::*;
use xylitol_ai_bridge::provider::langfuse_observation_properties_from;
use xylitol_ai_bridge::provider::trace::provider_trace_active;

/// Normalize Start/End reason strings to product kinds.
pub(crate) fn compaction_reason_kind(reason: &str) -> &'static str {
    if reason.starts_with("overflow") {
        "overflow"
    } else if reason.starts_with("threshold") {
        "threshold"
    } else {
        "manual"
    }
}

/// Lightweight marker for prepare-guard early exits (`agent.compaction.skipped`,
/// otel27).
///
/// otel19 reserves `agent.compaction` for post-prepare attempts; otel22 bars
/// gate early-exits from masquerading as LLM-lane spans — so this carries only
/// the skip reason and the session identity, never the llm lane, and the auto
/// path still emits no `CompactionEnd`.
pub(crate) fn export_skipped(
    reason_detail: &str,
    parent: Option<SpanContext>,
    obs: &xylitol_ai_bridge::ObsSessionContext,
) {
    if !provider_trace_active() {
        return;
    }
    let obs = obs.clone();
    let detail = reason_detail.to_string();
    let span = Span::root(
        "agent.compaction.skipped",
        parent.unwrap_or_else(SpanContext::random),
    )
    .with_properties(move || {
        let mut props = vec![("skip_reason".to_string(), detail)];
        props.extend(xylitol_ai_bridge::provider::langfuse_session_properties_from(&obs));
        props
    });
    span.add_event(Event::new("lifecycle").with_properties(|| {
        [
            ("kind", "lifecycle".to_string()),
            ("phase", "start".to_string()),
            ("name", "agent.compaction.skipped".to_string()),
        ]
    }));
    // Drop → flush via reporter (token.estimate template).
    drop(span);
}

/// Timed observation wrapping one compact attempt (`agent.compaction`).
pub(crate) struct AgentCompactionSpan {
    span: Span,
}

impl AgentCompactionSpan {
    /// Start after `prepare_compaction` succeeds (manual / threshold / overflow).
    ///
    /// `parent` is typically the active `agent.turn` context when compacting inside
    /// a turn; `None` starts an independent root (slash / out-of-turn compact).
    /// `obs` is the caller's session snapshot (otel24 / c2610) — never a mid-run
    /// read of the process slot.
    pub(crate) fn start(
        reason: &str,
        parent: Option<SpanContext>,
        obs: &xylitol_ai_bridge::ObsSessionContext,
    ) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let kind = compaction_reason_kind(reason);
        let parent = parent.unwrap_or_else(SpanContext::random);
        let obs = obs.clone();
        let span = Span::root("agent.compaction", parent).with_properties(move || {
            let mut props = vec![("reason".to_string(), kind.to_string())];
            if reason != kind {
                props.push(("reason.detail".to_string(), reason.to_string()));
            }
            props.extend(langfuse_observation_properties_from("span", &obs));
            props
        });
        span.add_event(Event::new("lifecycle").with_properties(|| {
            [
                ("kind", "lifecycle".to_string()),
                ("phase", "start".to_string()),
                ("name", "agent.compaction".to_string()),
                ("reason", kind.to_string()),
            ]
        }));
        Some(Self { span })
    }

    /// Captured parent context for summarization `llm.request` nesting.
    pub(crate) fn span_context(&self) -> Option<SpanContext> {
        SpanContext::from_span(&self.span)
    }

    /// Attach end-of-compact attributes then drop.
    pub(crate) fn finish(self, will_retry: bool, aborted: bool, error_message: Option<&str>) {
        self.span
            .add_property(|| ("will_retry", will_retry.to_string()));
        self.span.add_property(|| ("aborted", aborted.to_string()));
        if let Some(err) = error_message.filter(|s| !s.is_empty()) {
            let msg = if err.len() > 512 {
                format!("{}…", &err[..511])
            } else {
                err.to_string()
            };
            self.span
                .add_property(|| ("langfuse.observation.level", "ERROR".to_string()));
            self.span
                .add_property(|| ("langfuse.observation.status_message", msg.clone()));
            self.span.add_property(|| ("error_message", msg));
        }
        self.span
            .add_event(Event::new("lifecycle").with_properties(|| {
                [
                    ("kind", "lifecycle".to_string()),
                    ("phase", "end".to_string()),
                    ("name", "agent.compaction".to_string()),
                    ("will_retry", will_retry.to_string()),
                    ("aborted", aborted.to_string()),
                ]
            }));
        drop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use xylitol_ai_bridge::provider::obs_session::ObsSessionContext;
    use xylitol_ai_bridge::provider::trace::{ObsGateScope, ObsGateState, SpanCollectScope};

    #[test]
    fn reason_kind_maps() {
        assert_eq!(compaction_reason_kind("manual"), "manual");
        assert_eq!(compaction_reason_kind("overflow"), "overflow");
        assert_eq!(
            compaction_reason_kind("threshold: 100 tokens over reserve of 200k window"),
            "threshold"
        );
    }

    #[test]
    fn inactive_start_is_none() {
        let _g = ObsGateScope::enter(ObsGateState::OFF);
        assert!(
            AgentCompactionSpan::start("manual", None, &ObsSessionContext::default()).is_none()
        );
    }

    #[test]
    fn skipped_span_carries_session_without_lane() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();

        let snapshot = ObsSessionContext {
            session_id: Some("sess-skip-1".into()),
            session_name: None,
            ..Default::default()
        };
        export_skipped("Already compacted", None, &snapshot);
        fastrace::flush();

        let spans = collect.records();
        let skipped = spans
            .iter()
            .find(|s| s.name == "agent.compaction.skipped")
            .expect("skipped span");
        let props: std::collections::HashMap<_, _> = skipped
            .properties
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();
        assert_eq!(props.get("skip_reason"), Some(&"Already compacted"));
        assert_eq!(props.get("langfuse.session.id"), Some(&"sess-skip-1"));
        assert_eq!(props.get("xylitol.session.id"), Some(&"sess-skip-1"));
        assert!(
            !props.contains_key("xylitol.obs.lane"),
            "gate early-exit must not carry the llm lane (otel27/otel22)"
        );
    }

    #[test]
    fn skipped_span_silent_when_gate_off() {
        let _g = ObsGateScope::enter(ObsGateState::OFF);
        let collect = SpanCollectScope::enter();
        export_skipped("Already compacted", None, &ObsSessionContext::default());
        fastrace::flush();
        assert!(
            collect
                .records()
                .iter()
                .all(|s| s.name != "agent.compaction.skipped")
        );
    }

    #[test]
    fn compaction_under_turn_shares_trace() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();

        {
            let turn = Span::root("agent.turn", SpanContext::random());
            let turn_ctx = SpanContext::from_span(&turn).expect("turn ctx");
            let c = AgentCompactionSpan::start(
                "threshold: demo",
                Some(turn_ctx),
                &ObsSessionContext::default(),
            )
            .expect("compact");
            c.finish(false, false, None);
            drop(turn);
        }
        fastrace::flush();

        let spans = collect.records();
        let turn = spans.iter().find(|s| s.name == "agent.turn").expect("turn");
        let compact = spans
            .iter()
            .find(|s| s.name == "agent.compaction")
            .expect("compaction");
        assert_eq!(compact.trace_id, turn.trace_id);
        assert_eq!(compact.parent_id, turn.span_id);
        let props: std::collections::HashMap<_, _> = compact
            .properties
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();
        assert_eq!(props.get("reason"), Some(&"threshold"));
        assert_eq!(props.get("langfuse.observation.type"), Some(&"span"));
        assert_eq!(
            props.get("xylitol.obs.lane"),
            Some(&xylitol_ai_bridge::provider::XYLITOL_OBS_LANE_LLM)
        );
        assert_eq!(props.get("will_retry"), Some(&"false"));
        assert_eq!(props.get("aborted"), Some(&"false"));
    }

    #[test]
    fn independent_root_carries_session_id_and_lane() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();

        let snapshot = ObsSessionContext {
            session_id: Some("sess-compact-1".into()),
            session_name: None,
            ..Default::default()
        };

        {
            // Post-prepare failure path (e.g. summarization error), not prepare early-exit.
            let c = AgentCompactionSpan::start("manual", None, &snapshot).expect("compact");
            c.finish(false, false, Some("compaction failed: model error"));
        }
        fastrace::flush();

        let spans = collect.records();
        let compact = spans
            .iter()
            .find(|s| s.name == "agent.compaction")
            .expect("compaction");
        let props: std::collections::HashMap<_, _> = compact
            .properties
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();
        assert_eq!(props.get("reason"), Some(&"manual"));
        assert_eq!(props.get("langfuse.session.id"), Some(&"sess-compact-1"));
        assert_eq!(props.get("xylitol.session.id"), Some(&"sess-compact-1"));
        assert!(!props.contains_key("xylitol.session.llm_gateway_session_id"));
        assert_eq!(
            props.get("xylitol.obs.lane"),
            Some(&xylitol_ai_bridge::provider::XYLITOL_OBS_LANE_LLM)
        );
        assert_eq!(props.get("langfuse.observation.level"), Some(&"ERROR"));
        assert!(
            props
                .get("langfuse.observation.status_message")
                .is_some_and(|m| m.contains("compaction failed"))
        );
    }

    #[test]
    fn summarization_llm_nests_under_compaction() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();

        {
            let turn = Span::root("agent.turn", SpanContext::random());
            let turn_ctx = SpanContext::from_span(&turn);
            let c = AgentCompactionSpan::start("overflow", turn_ctx, &ObsSessionContext::default())
                .expect("compact");
            let compact_ctx = c.span_context();
            let _llm = xylitol_ai_bridge::provider::trace::ProviderRequestTrace::start_with_parent(
                "openai-responses",
                "m",
                compact_ctx,
            )
            .expect("llm");
            drop(_llm);
            c.finish(true, false, None);
            drop(turn);
        }
        fastrace::flush();

        let spans = collect.records();
        let compact = spans
            .iter()
            .find(|s| s.name == "agent.compaction")
            .expect("compaction");
        let llm = spans.iter().find(|s| s.name == "llm.request").expect("llm");
        assert_eq!(llm.trace_id, compact.trace_id);
        assert_eq!(llm.parent_id, compact.span_id);
        let props: std::collections::HashMap<_, _> = compact
            .properties
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();
        assert_eq!(props.get("reason"), Some(&"overflow"));
        assert_eq!(props.get("will_retry"), Some(&"true"));
    }
}
