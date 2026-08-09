//! Fastrace / Langfuse span for session compaction (`agent.compaction`, c1700).
//!
//! Lives here (not `agent/runtime/obs`) so `CompactionOrchestrator` can emit without
//! a runtime ↔ compaction cycle. Gated like `token.estimate` / ReAct low-freq spans.

use fastrace::prelude::*;
use xylitol_ai_bridge::provider::langfuse_observation_properties;
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

/// Timed observation wrapping one compact attempt (`agent.compaction`).
pub(crate) struct AgentCompactionSpan {
    span: Span,
}

impl AgentCompactionSpan {
    /// Start after `prepare_compaction` succeeds (manual / threshold / overflow).
    ///
    /// `parent` is typically the active `agent.turn` context when compacting inside
    /// a turn; `None` starts an independent root (slash / out-of-turn compact).
    pub(crate) fn start(reason: &str, parent: Option<SpanContext>) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let kind = compaction_reason_kind(reason);
        let parent = parent.unwrap_or_else(SpanContext::random);
        let span = Span::root("agent.compaction", parent).with_properties(|| {
            let mut props = vec![("reason".to_string(), kind.to_string())];
            if reason != kind {
                props.push(("reason.detail".to_string(), reason.to_string()));
            }
            props.extend(langfuse_observation_properties("span"));
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
    use serial_test::serial;
    use std::sync::{Arc, Mutex};

    use fastrace::collector::{Config, Reporter, SpanRecord};
    use xylitol_ai_bridge::provider::obs_session::{clear_obs_session, set_obs_session};
    use xylitol_ai_bridge::provider::trace::set_provider_trace_active;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct CollectingReporter(Arc<Mutex<Vec<SpanRecord>>>);

    impl Reporter for CollectingReporter {
        fn report(&mut self, spans: Vec<SpanRecord>) {
            self.0.lock().unwrap().extend(spans);
        }
    }

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
    #[serial(obs_global)]
    fn inactive_start_is_none() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(false);
        assert!(AgentCompactionSpan::start("manual", None).is_none());
    }

    #[test]
    #[serial(obs_global)]
    fn compaction_under_turn_shares_trace() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let turn = Span::root("agent.turn", SpanContext::random());
            let turn_ctx = SpanContext::from_span(&turn).expect("turn ctx");
            let c = AgentCompactionSpan::start("threshold: demo", Some(turn_ctx)).expect("compact");
            c.finish(false, false, None);
            drop(turn);
        }
        fastrace::flush();
        set_provider_trace_active(false);

        let spans = records.lock().unwrap().clone();
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
    #[serial(obs_global)]
    fn independent_root_carries_session_id_and_lane() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        clear_obs_session();
        set_obs_session("sess-compact-1", None);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            // Post-prepare failure path (e.g. summarization error), not prepare early-exit.
            let c = AgentCompactionSpan::start("manual", None).expect("compact");
            c.finish(false, false, Some("compaction failed: model error"));
        }
        fastrace::flush();
        clear_obs_session();
        set_provider_trace_active(false);

        let spans = records.lock().unwrap().clone();
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
    #[serial(obs_global)]
    fn summarization_llm_nests_under_compaction() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let turn = Span::root("agent.turn", SpanContext::random());
            let turn_ctx = SpanContext::from_span(&turn);
            let c = AgentCompactionSpan::start("overflow", turn_ctx).expect("compact");
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
        set_provider_trace_active(false);

        let spans = records.lock().unwrap().clone();
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
