//! Low-frequency ReAct fastrace spans (c1265 Phase A; c1495 parent tree).
//!
//! Gated by [`xylitol_ai_bridge::provider::trace::provider_trace_active`]. When
//! off, helpers are no-ops (zero/near-zero cost) — no UUID, no parent-slot writes.
//!
//! Hierarchy (exported names):
//! ```text
//! agent.turn
//!   ├─ agent.iteration
//!   │    ├─ llm.request   (via bridge obs parent slot)
//!   │    └─ tool.execute
//!   └─ agent.compaction   (via agent/compaction/obs; may parent summarization llm.request)
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
use xylitol_ai_bridge::provider::trace::{
    observation_io_tier, provider_trace_active, tool_observation_io_tier, truncate_observation_text,
};

use crate::protocol::error::{XyError, XyToolError};

/// How an [`AgentTurnSpan`] ended (c1720 / otel20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnEndReason {
    /// Normal completion (or non-abort early exit). MUST NOT mark ERROR.
    Ok,
    /// User cancel / cancel-token abort. Marks Langfuse ERROR + `aborted`.
    Aborted,
}

/// Root span for one user-triggered agent run (`agent.turn`).
pub(crate) struct AgentTurnSpan {
    root: Span,
    turn_id: String,
}

impl AgentTurnSpan {
    /// Start a turn root. When `[otel].observation_io` ≠ none, `user_preview` is
    /// attached as `langfuse.observation.input` for Langfuse Session list (c1555).
    /// `model_api` (when known) is recorded as `xylitol.model.api` (c1600).
    pub(crate) fn start(user_preview: Option<&str>, model_api: Option<&str>) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let turn_id = uuid::Uuid::new_v4().to_string();
        let root = Span::root("agent.turn", SpanContext::random()).with_properties(|| {
            let mut props = vec![("turn_id".to_string(), turn_id.clone())];
            props.extend(langfuse_observation_properties("agent"));
            if let Some(api) = model_api.filter(|s| !s.is_empty()) {
                props.push(("xylitol.model.api".to_string(), api.to_string()));
            }
            if let Some(max) = observation_io_tier().max_chars()
                && let Some(preview) = user_preview
                && !preview.is_empty()
            {
                let (s, _) = truncate_observation_text(preview, max);
                props.push(("langfuse.observation.input".to_string(), s));
            }
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

    /// Attach terminal status then drop (clears turn parent). Abort → ERROR/`aborted`
    /// (aligned with `agent.compaction` / generation abort). Ok → no ERROR level.
    pub(crate) fn finish(self, reason: TurnEndReason) {
        match reason {
            TurnEndReason::Aborted => {
                self.root
                    .add_property(|| ("langfuse.observation.level", "ERROR".to_string()));
                self.root.add_property(|| {
                    ("langfuse.observation.status_message", "aborted".to_string())
                });
            }
            TurnEndReason::Ok => {}
        }
        let end = match reason {
            TurnEndReason::Ok => "ok",
            TurnEndReason::Aborted => "aborted",
        };
        self.root
            .add_event(Event::new("lifecycle").with_properties(|| {
                [
                    ("kind", "lifecycle".to_string()),
                    ("phase", "end".to_string()),
                    ("name", "agent.turn".to_string()),
                    ("end_reason", end.to_string()),
                ]
            }));
        drop(self);
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
    span: Span,
}

impl ToolExecuteSpan {
    #[cfg(test)]
    pub(crate) fn start(name: &str, id: &str, parent: Option<&Span>) -> Option<Self> {
        Self::start_with_batch(name, id, parent, None, None)
    }

    #[cfg(test)]
    pub(crate) fn start_with_batch(
        name: &str,
        id: &str,
        parent: Option<&Span>,
        batch_mode: Option<&str>,
        barrier_index: Option<u32>,
    ) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let span = match parent {
            Some(p) => Span::enter_with_parent("tool.execute", p),
            None => Span::root("tool.execute", SpanContext::random()),
        };
        Some(Self::finish_start(
            span,
            name,
            id,
            batch_mode,
            barrier_index,
        ))
    }

    /// Child of a captured parent [`SpanContext`] (BarrierParallel fan-out).
    ///
    /// Capturing the context before `join_all` avoids racing the global parent
    /// slot and avoids `SpanContext::random` roots for concurrent tools.
    pub(crate) fn start_with_parent_ctx(
        name: &str,
        id: &str,
        parent_ctx: Option<SpanContext>,
        batch_mode: &str,
        barrier_index: u32,
    ) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let span = match parent_ctx {
            Some(ctx) => Span::root("tool.execute", ctx),
            None => Span::root("tool.execute", SpanContext::random()),
        };
        Some(Self::finish_start(
            span,
            name,
            id,
            Some(batch_mode),
            Some(barrier_index),
        ))
    }

    fn finish_start(
        span: Span,
        name: &str,
        id: &str,
        batch_mode: Option<&str>,
        barrier_index: Option<u32>,
    ) -> Self {
        let span = span.with_properties(|| {
            let mut props = vec![
                ("tool_name".to_string(), name.to_string()),
                ("tool_id".to_string(), id.to_string()),
            ];
            if let Some(m) = batch_mode {
                props.push(("tool_batch.mode".to_string(), m.to_string()));
            }
            if let Some(i) = barrier_index {
                props.push(("tool_batch.barrier_index".to_string(), i.to_string()));
            }
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
        Self { span }
    }

    /// Attach args/result when `[otel].tool_observation_io` ≠ none (c1550).
    pub(crate) fn attach_io(&self, input: &str, output: &str) {
        let Some(max) = tool_observation_io_tier().max_chars() else {
            return;
        };
        let (inn, _) = truncate_observation_text(input, max);
        let (out, _) = truncate_observation_text(output, max);
        self.span
            .add_property(|| ("langfuse.observation.input", inn));
        self.span
            .add_property(|| ("langfuse.observation.output", out));
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
    use serial_test::serial;
    use std::sync::{Arc, Mutex};

    use fastrace::collector::{Config, Reporter, SpanRecord};
    use xylitol_ai_bridge::provider::trace::{
        ObservationIoTier, set_observation_io_tier, set_provider_trace_active,
        set_tool_observation_io_tier,
    };

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct CollectingReporter(Arc<Mutex<Vec<SpanRecord>>>);

    impl Reporter for CollectingReporter {
        fn report(&mut self, spans: Vec<SpanRecord>) {
            self.0.lock().unwrap().extend(spans);
        }
    }

    #[test]
    #[serial(obs_global)]

    fn inactive_helpers_are_none() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(false);
        assert!(AgentTurnSpan::start(None, None).is_none());
        assert!(AgentIterationSpan::start(None, 0).is_none());
        assert!(ToolExecuteSpan::start("bash", "1", None).is_none());
    }

    #[test]
    #[serial(obs_global)]

    fn turn_finish_ok_has_no_error_level() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        clear_obs_span_parents();
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let turn = AgentTurnSpan::start(Some("hello"), None).expect("turn");
            turn.finish(TurnEndReason::Ok);
        }
        fastrace::flush();
        set_provider_trace_active(false);

        let spans = records.lock().unwrap().clone();
        let turn = spans
            .iter()
            .find(|s| s.name == "agent.turn")
            .expect("turn span");
        let props: std::collections::HashMap<_, _> = turn
            .properties
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();
        assert!(
            !props.contains_key("langfuse.observation.level"),
            "ok finish must not mark ERROR: {props:?}"
        );
        assert!(
            !props.contains_key("langfuse.observation.status_message"),
            "ok finish must not set status_message: {props:?}"
        );
    }

    #[test]
    #[serial(obs_global)]

    fn turn_finish_aborted_marks_error() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        clear_obs_span_parents();
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let turn = AgentTurnSpan::start(Some("hello"), None).expect("turn");
            turn.finish(TurnEndReason::Aborted);
        }
        fastrace::flush();
        set_provider_trace_active(false);

        let spans = records.lock().unwrap().clone();
        let turn = spans
            .iter()
            .find(|s| s.name == "agent.turn")
            .expect("turn span");
        let props: std::collections::HashMap<_, _> = turn
            .properties
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();
        assert_eq!(props.get("langfuse.observation.level"), Some(&"ERROR"));
        assert_eq!(
            props.get("langfuse.observation.status_message"),
            Some(&"aborted")
        );
    }

    #[test]
    #[serial(obs_global)]

    fn turn_iteration_llm_share_trace_id() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        clear_obs_span_parents();
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let turn = AgentTurnSpan::start(Some("hello turn"), None).expect("turn");
            let iter = AgentIterationSpan::start(Some(&turn), 0).expect("iter");
            let tool = ToolExecuteSpan::start("bash", "t1", Some(iter.span())).expect("tool");
            tool.attach_io(r#"{"cmd":"echo"}"#, "ok");
            let _llm = xylitol_ai_bridge::provider::trace::ProviderRequestTrace::start(
                "openai-responses",
                "m",
            )
            .expect("llm");
            drop(_llm);
            drop(tool);
            drop(iter);
            turn.finish(TurnEndReason::Ok);
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

    #[test]
    #[serial(obs_global)]

    fn turn_root_input_only_when_observation_io_set() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        clear_obs_span_parents();
        set_observation_io_tier(ObservationIoTier::None);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());
        {
            let turn = AgentTurnSpan::start(Some("secret prompt"), None).expect("turn");
            turn.finish(TurnEndReason::Ok);
        }
        fastrace::flush();
        {
            let spans = records.lock().unwrap_or_else(|e| e.into_inner());
            let turn = spans.iter().find(|s| s.name == "agent.turn").expect("turn");
            assert!(
                !turn
                    .properties
                    .iter()
                    .any(|(k, _)| k.as_ref() == "langfuse.observation.input"),
                "none tier must not write turn input"
            );
        }
        records.lock().unwrap().clear();
        set_observation_io_tier(ObservationIoTier::Truncated);
        {
            let turn = AgentTurnSpan::start(Some("secret prompt"), None).expect("turn");
            turn.finish(TurnEndReason::Ok);
        }
        fastrace::flush();
        set_observation_io_tier(ObservationIoTier::None);
        set_provider_trace_active(false);
        let spans = records.lock().unwrap().clone();
        let turn = spans.iter().find(|s| s.name == "agent.turn").expect("turn");
        let input = turn
            .properties
            .iter()
            .find(|(k, _)| k.as_ref() == "langfuse.observation.input")
            .map(|(_, v)| v.as_ref());
        assert_eq!(input, Some("secret prompt"));
    }

    #[test]
    #[serial(obs_global)]

    fn tool_io_only_when_tool_observation_io_set() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        clear_obs_span_parents();
        set_tool_observation_io_tier(ObservationIoTier::None);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());
        {
            let tool = ToolExecuteSpan::start("bash", "t1", None).expect("tool");
            tool.attach_io(r#"{"x":1}"#, "out");
            drop(tool);
        }
        fastrace::flush();
        {
            let spans = records.lock().unwrap_or_else(|e| e.into_inner());
            let tool = spans
                .iter()
                .find(|s| s.name == "tool.execute")
                .expect("tool");
            assert!(
                !tool.properties.iter().any(|(k, _)| {
                    matches!(
                        k.as_ref(),
                        "langfuse.observation.input" | "langfuse.observation.output"
                    )
                }),
                "none tier must not write tool I/O"
            );
        }
        records.lock().unwrap().clear();
        set_tool_observation_io_tier(ObservationIoTier::Truncated);
        {
            let tool = ToolExecuteSpan::start("bash", "t2", None).expect("tool");
            tool.attach_io(r#"{"x":1}"#, "out");
            drop(tool);
        }
        fastrace::flush();
        set_tool_observation_io_tier(ObservationIoTier::None);
        set_provider_trace_active(false);
        let spans = records.lock().unwrap().clone();
        let tool = spans
            .iter()
            .find(|s| s.name == "tool.execute")
            .expect("tool");
        let keys: Vec<&str> = tool.properties.iter().map(|(k, _)| k.as_ref()).collect();
        assert!(keys.contains(&"langfuse.observation.input"), "{keys:?}");
        assert!(keys.contains(&"langfuse.observation.output"), "{keys:?}");
    }

    #[test]
    #[serial(obs_global)]

    fn parallel_tool_spans_share_iteration_parent_via_captured_ctx() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_provider_trace_active(true);
        clear_obs_span_parents();
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let turn = AgentTurnSpan::start(None, Some("openai-responses")).expect("turn");
            let iter = AgentIterationSpan::start(Some(&turn), 1).expect("iter");
            let parent_ctx = SpanContext::from_span(iter.span());
            let t1 = ToolExecuteSpan::start_with_parent_ctx(
                "slow_safe",
                "c0",
                parent_ctx,
                "barrier_parallel",
                0,
            )
            .expect("t1");
            let t2 = ToolExecuteSpan::start_with_parent_ctx(
                "slow_safe",
                "c1",
                parent_ctx,
                "barrier_parallel",
                0,
            )
            .expect("t2");
            drop(t1);
            drop(t2);
            drop(iter);
            turn.finish(TurnEndReason::Ok);
        }
        fastrace::flush();
        set_provider_trace_active(false);

        let spans = records.lock().unwrap().clone();
        let turn = spans.iter().find(|s| s.name == "agent.turn").expect("turn");
        let turn_props: std::collections::HashMap<_, _> = turn
            .properties
            .iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();
        assert_eq!(
            turn_props.get("xylitol.model.api"),
            Some(&"openai-responses")
        );
        let iter = spans
            .iter()
            .find(|s| s.name == "agent.iteration")
            .expect("iter");
        let tools: Vec<_> = spans.iter().filter(|s| s.name == "tool.execute").collect();
        assert_eq!(tools.len(), 2);
        for t in &tools {
            assert_eq!(t.trace_id, iter.trace_id);
            assert_eq!(t.parent_id, iter.span_id);
            let props: std::collections::HashMap<_, _> = t
                .properties
                .iter()
                .map(|(k, v)| (k.as_ref(), v.as_ref()))
                .collect();
            assert_eq!(props.get("tool_batch.mode"), Some(&"barrier_parallel"));
            assert_eq!(props.get("tool_batch.barrier_index"), Some(&"0"));
        }
    }
}
