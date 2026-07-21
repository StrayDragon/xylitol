//! Low-frequency ReAct fastrace spans (c1265 Phase A).
//!
//! Gated by [`xylitol_ai_bridge::provider::trace::provider_trace_active`]. When
//! off, helpers are no-ops (zero/near-zero cost).
//!
//! Note: do **not** hold [`LocalParentGuard`] across `.await` in the ReAct
//! `async_stream` (guard is `!Send`). `fastrace-futures::in_span` sets local
//! parent only during sync `poll_next`. Must not import `crate::infra` (agent
//! stays on ports / bridge; see `src/AGENTS.md`).

use std::pin::Pin;

use fastrace::prelude::*;
use fastrace_futures::StreamExt as _;
use futures::Stream;
use xylitol_ai_bridge::provider::trace::provider_trace_active;

use crate::protocol::error::{XyError, XyToolError};
use crate::protocol::types::XyChunk;

type ChunkStream = Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;

/// Root span for one ReAct iteration; drop when the turn ends.
pub(crate) struct ReactTurnSpan {
    _root: Span,
    turn_id: String,
}

impl ReactTurnSpan {
    pub(crate) fn start(turn_index: usize) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let turn_id = uuid::Uuid::new_v4().to_string();
        let root = Span::root("react.turn", SpanContext::random()).with_properties(|| {
            [
                ("turn_id", turn_id.clone()),
                ("turn_index", turn_index.to_string()),
            ]
        });
        root.add_event(Event::new("lifecycle").with_properties(|| {
            [
                ("kind", "lifecycle".to_string()),
                ("phase", "start".to_string()),
                ("name", "react.turn".to_string()),
            ]
        }));
        Some(Self {
            _root: root,
            turn_id,
        })
    }

    pub(crate) fn turn_id(&self) -> &str {
        &self.turn_id
    }
}

fn stream_span(turn_id: Option<&str>) -> Span {
    let tid = turn_id.unwrap_or("").to_string();
    // Always root: cannot rely on local parent across await boundaries.
    let span = Span::root("react.stream", SpanContext::random())
        .with_properties(|| [("turn_id", tid), ("span_role", "react.stream".to_string())]);
    span.add_event(Event::new("lifecycle").with_properties(|| {
        [
            ("kind", "lifecycle".to_string()),
            ("phase", "start".to_string()),
            ("name", "react.stream".to_string()),
        ]
    }));
    span
}

/// Wrap a provider chunk stream in `react.stream` via fastrace-futures `in_span`.
pub(crate) fn wrap_chunk_stream(stream: ChunkStream, turn_id: Option<&str>) -> ChunkStream {
    if !provider_trace_active() {
        return stream;
    }
    Box::pin(stream.in_span(stream_span(turn_id)))
}

/// Span around a single tool execution (dropped at end of the tool loop body).
pub(crate) struct ToolExecuteSpan {
    _span: Span,
}

impl ToolExecuteSpan {
    pub(crate) fn start(name: &str, id: &str) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let span = Span::root("tool.execute", SpanContext::random())
            .with_properties(|| [("tool_name", name.to_string()), ("tool_id", id.to_string())]);
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
    let span = Span::root("react.error", SpanContext::random()).with_properties(|| {
        [
            ("error.kind", kind.to_string()),
            ("where", where_.to_string()),
            ("turn_id", tid.to_string()),
        ]
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
    let span = Span::root("tool.error", SpanContext::random()).with_properties(|| {
        [
            ("error.kind", kind.to_string()),
            ("tool_name", tool.to_string()),
            ("turn_id", tid.to_string()),
        ]
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
    use xylitol_ai_bridge::provider::trace::set_provider_trace_active;

    #[test]
    fn inactive_helpers_are_none() {
        set_provider_trace_active(false);
        assert!(ReactTurnSpan::start(0).is_none());
        assert!(ToolExecuteSpan::start("bash", "1").is_none());
    }
}
