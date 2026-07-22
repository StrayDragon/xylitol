//! Low-frequency ReAct fastrace spans (c1265 Phase A; c1495 parent tree).
//!
//! Gated by [`xylitol_ai_bridge::provider::trace::provider_trace_active`]. When
//! off, helpers are no-ops (zero/near-zero cost) — no UUID, no parent-slot writes.
//!
//! Hierarchy (exported names):
//! ```text
//! agent.turn
//!   └─ agent.iteration
//!        ├─ llm.request   (via bridge obs parent slot)
//!        └─ tool.execute
//! ```
//!
//! Do **not** hold [`LocalParentGuard`] across `.await` in the ReAct
//! `async_stream` (guard is `!Send`). Children use `enter_with_parent` or
//! `Span::root(name, SpanContext::from_span(parent))` with an owned parent
//! `Span` kept alive for the turn/iteration. Must not import `crate::infra`.

use fastrace::prelude::*;
use xylitol_ai_bridge::provider::langfuse_observation_properties;
use xylitol_ai_bridge::provider::obs_span_parent::{
    clear_obs_span_parents, obs_llm_parent, set_obs_iteration_parent, set_obs_turn_parent,
};
use xylitol_ai_bridge::provider::trace::provider_trace_active;

use crate::protocol::error::{XyError, XyToolError};

/// Root span for one user-triggered agent run (`agent.turn`).
pub(crate) struct AgentTurnSpan {
    root: Span,
    turn_id: String,
}

impl AgentTurnSpan {
    pub(crate) fn start() -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let turn_id = uuid::Uuid::new_v4().to_string();
        let root = Span::root("agent.turn", SpanContext::random()).with_properties(|| {
            let mut props = vec![("turn_id".to_string(), turn_id.clone())];
            props.extend(langfuse_observation_properties("agent"));
            props
        });
        root.add_event(Event::new("lifecycle").with_properties(|| {
            [
                ("kind", "lifecycle".to_string()),
                ("phase", "start".to_string()),
                ("name", "agent.turn".to_string()),
            ]
        }));
        if let Some(ctx) = SpanContext::from_span(&root) {
            set_obs_turn_parent(Some(ctx));
        }
        Some(Self { root, turn_id })
    }

    pub(crate) fn turn_id(&self) -> &str {
        &self.turn_id
    }

    pub(crate) fn span(&self) -> &Span {
        &self.root
    }
}

impl Drop for AgentTurnSpan {
    fn drop(&mut self) {
        clear_obs_span_parents();
    }
}

/// One ReAct loop step under [`AgentTurnSpan`] (`agent.iteration`).
pub(crate) struct AgentIterationSpan {
    span: Span,
    turn_id: String,
}

impl AgentIterationSpan {
    pub(crate) fn start(turn: Option<&AgentTurnSpan>, turn_index: usize) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let turn_id = turn
            .map(|t| t.turn_id().to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let span = match turn {
            Some(t) => Span::enter_with_parent("agent.iteration", t.span()),
            None => Span::root("agent.iteration", SpanContext::random()),
        }
        .with_properties(|| {
            // `turn_id` lives on `agent.turn` span attrs only — same value on every
            // iteration was Langfuse observation noise (parent chain correlates).
            let mut props = vec![("turn_index".to_string(), turn_index.to_string())];
            props.extend(langfuse_observation_properties("agent"));
            props
        });
        span.add_event(Event::new("lifecycle").with_properties(|| {
            [
                ("kind", "lifecycle".to_string()),
                ("phase", "start".to_string()),
                ("name", "agent.iteration".to_string()),
                // Local JSONL / inspect correlation only (not a span attribute).
                ("turn_id", turn_id.clone()),
            ]
        }));
        if let Some(ctx) = SpanContext::from_span(&span) {
            set_obs_iteration_parent(Some(ctx));
        }
        Some(Self { span, turn_id })
    }

    pub(crate) fn turn_id(&self) -> &str {
        &self.turn_id
    }

    pub(crate) fn span(&self) -> &Span {
        &self.span
    }
}

impl Drop for AgentIterationSpan {
    fn drop(&mut self) {
        set_obs_iteration_parent(None);
    }
}

/// Span around a single tool execution (child of iteration when provided).
pub(crate) struct ToolExecuteSpan {
    _span: Span,
}

impl ToolExecuteSpan {
    pub(crate) fn start(name: &str, id: &str, parent: Option<&Span>) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let span = match parent {
            Some(p) => Span::enter_with_parent("tool.execute", p),
            None => Span::root("tool.execute", SpanContext::random()),
        }
        .with_properties(|| {
            let mut props = vec![
                ("tool_name".to_string(), name.to_string()),
                ("tool_id".to_string(), id.to_string()),
            ];
            props.extend(langfuse_observation_properties("tool"));
            props
        });
        span.add_event(Event::new("lifecycle").with_properties(|| {
            [
                ("kind", "lifecycle".to_string()),
                ("phase", "start".to_string()),
                ("name", "tool.execute".to_string()),
            ]
        }));
        Some(Self { _span: span })
    }
}

/// Log (+ optional fastrace) a hot-path [`XyError`] with stable `error.kind`.
pub(crate) fn record_xy_error(where_: &str, err: &XyError, turn_id: Option<&str>) {
    let kind = err.kind();
    let tid = turn_id.unwrap_or("");
    log::warn!(
        target: "xylitol::react",
        "{where_} failed error.kind={kind} turn_id={tid} error={err}"
    );
    if !provider_trace_active() {
        return;
    }
    let parent = obs_llm_parent().unwrap_or_else(SpanContext::random);
    let span = Span::root("react.error", parent).with_properties(|| {
        let mut props = vec![
            ("error.kind".to_string(), kind.to_string()),
            ("where".to_string(), where_.to_string()),
            ("turn_id".to_string(), tid.to_string()),
        ];
        props.extend(langfuse_observation_properties("span"));
        props
    });
    span.add_event(Event::new("error").with_properties(|| {
        [
            ("error.kind", kind.to_string()),
            ("where", where_.to_string()),
            ("message", err.to_string()),
        ]
    }));
}

/// Log (+ optional fastrace) a tool failure with stable `error.kind`.
pub(crate) fn record_tool_error(tool: &str, err: &XyToolError, turn_id: Option<&str>) {
    let kind = err.kind();
    let tid = turn_id.unwrap_or("");
    log::warn!(
        target: "xylitol::react",
        "tool.execute failed tool={tool} error.kind={kind} turn_id={tid} error={err}"
    );
    if !provider_trace_active() {
        return;
    }
    let parent = obs_llm_parent().unwrap_or_else(SpanContext::random);
    let span = Span::root("tool.error", parent).with_properties(|| {
        let mut props = vec![
            ("error.kind".to_string(), kind.to_string()),
            ("tool_name".to_string(), tool.to_string()),
            ("turn_id".to_string(), tid.to_string()),
        ];
        props.extend(langfuse_observation_properties("span"));
        props
    });
    span.add_event(Event::new("error").with_properties(|| {
        [
            ("error.kind", kind.to_string()),
            ("tool_name", tool.to_string()),
            ("message", err.to_string()),
        ]
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use fastrace::collector::{Config, Reporter, SpanRecord};
    use xylitol_ai_bridge::provider::trace::set_provider_trace_active;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct CollectingReporter(Arc<Mutex<Vec<SpanRecord>>>);

    impl Reporter for CollectingReporter {
        fn report(&mut self, spans: Vec<SpanRecord>) {
            self.0.lock().unwrap().extend(spans);
        }
    }

    #[test]
    fn inactive_helpers_are_none() {
        let _g = TEST_LOCK.lock().unwrap();
        set_provider_trace_active(false);
        assert!(AgentTurnSpan::start().is_none());
        assert!(AgentIterationSpan::start(None, 0).is_none());
        assert!(ToolExecuteSpan::start("bash", "1", None).is_none());
    }

    #[test]
    fn turn_iteration_llm_share_trace_id() {
        let _g = TEST_LOCK.lock().unwrap();
        set_provider_trace_active(true);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let turn = AgentTurnSpan::start().expect("turn");
            let iter = AgentIterationSpan::start(Some(&turn), 0).expect("iter");
            let _tool = ToolExecuteSpan::start("bash", "t1", Some(iter.span()));
            let _llm = xylitol_ai_bridge::provider::trace::ProviderRequestTrace::start(
                "openai-responses",
                "m",
            )
            .expect("llm");
            drop(_llm);
            drop(_tool);
            drop(iter);
            drop(turn);
        }
        fastrace::flush();
        set_provider_trace_active(false);

        let spans = records.lock().unwrap().clone();
        let turn = spans
            .iter()
            .find(|s| s.name == "agent.turn")
            .expect("turn span");
        let iter = spans
            .iter()
            .find(|s| s.name == "agent.iteration")
            .expect("iter span");
        let llm = spans
            .iter()
            .find(|s| s.name == "llm.request")
            .expect("llm span");
        let tool = spans
            .iter()
            .find(|s| s.name == "tool.execute")
            .expect("tool span");

        assert_eq!(iter.trace_id, turn.trace_id);
        assert_eq!(llm.trace_id, turn.trace_id);
        assert_eq!(tool.trace_id, turn.trace_id);
        assert_eq!(iter.parent_id, turn.span_id);
        assert_eq!(tool.parent_id, iter.span_id);
        assert_eq!(llm.parent_id, iter.span_id);
        // Attribute slim: single model key; no duplicate turn_id on iteration.
        let llm_keys: Vec<&str> = llm.properties.iter().map(|(k, _)| k.as_ref()).collect();
        assert!(
            llm_keys.contains(&"langfuse.observation.model.name"),
            "llm keys={llm_keys:?}"
        );
        assert!(
            !llm_keys
                .iter()
                .any(|k| *k == "model" || *k == "gen_ai.request.model")
        );
        let iter_keys: Vec<&str> = iter.properties.iter().map(|(k, _)| k.as_ref()).collect();
        assert!(iter_keys.contains(&"turn_index"), "{iter_keys:?}");
        assert!(
            !iter_keys.contains(&"turn_id"),
            "iteration must not copy turn_id: {iter_keys:?}"
        );
        assert!(!spans.iter().any(|s| {
            matches!(
                s.name.as_ref(),
                "react.turn" | "react.stream" | "provider.request"
            )
        }));
    }
}
