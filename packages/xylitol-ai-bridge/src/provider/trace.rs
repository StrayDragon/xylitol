//! Provider request timeline helpers (fastrace Events → provider-trace.jsonl).
//!
//! Runtime gate via [`provider_trace_active`]; when inactive, emit helpers are
//! no-ops and must not allocate large SSE strings.

use std::sync::atomic::{AtomicBool, Ordering};

use fastrace::prelude::*;

use crate::dto::AiBridgeChunk;

/// Max Unicode scalars for `text` fields in provider-trace JSONL (c1000).
pub const PROVIDER_TRACE_TEXT_MAX: usize = 4096;

static PROVIDER_TRACE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Called from composition-root logging init when the FileReporter is installed.
pub fn set_provider_trace_active(active: bool) {
    PROVIDER_TRACE_ACTIVE.store(active, Ordering::Relaxed);
}

#[inline]
pub fn provider_trace_active() -> bool {
    PROVIDER_TRACE_ACTIVE.load(Ordering::Relaxed)
}

/// Root span for one provider HTTP stream; drop (end of stream) reports to FileReporter.
pub struct ProviderRequestTrace {
    root: Span,
    #[allow(dead_code)]
    request_id: String,
}

impl ProviderRequestTrace {
    pub fn start(api: &str, model: &str) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let root = Span::root("provider.request", SpanContext::random()).with_properties(|| {
            [
                ("request_id", request_id.clone()),
                ("api", api.to_string()),
                ("model", model.to_string()),
            ]
        });
        Some(Self { root, request_id })
    }

    pub fn emit_raw(&self, event: &str, text: &str) {
        if !provider_trace_active() {
            return;
        }
        let (text, truncated) = truncate_text(text);
        self.root.add_event(Event::new("raw").with_properties(|| {
            [
                ("kind", "raw".into()),
                ("event", event.to_string()),
                ("text", text),
                ("truncated", if truncated { "true" } else { "false" }.into()),
            ]
        }));
    }

    pub fn emit_mapped_chunk(&self, chunk: &AiBridgeChunk) {
        if !provider_trace_active() {
            return;
        }
        let (variant, text) = match chunk {
            AiBridgeChunk::TextDelta(t) => ("TextDelta", t.as_str()),
            AiBridgeChunk::ThinkingDelta(t) => ("ThinkingDelta", t.as_str()),
            AiBridgeChunk::ThinkingEnd { thinking, .. } => ("ThinkingEnd", thinking.as_str()),
            AiBridgeChunk::ToolCallStart { name, .. } => ("ToolCallStart", name.as_str()),
            AiBridgeChunk::ToolCallDelta { name, .. } => ("ToolCallDelta", name.as_str()),
            AiBridgeChunk::ToolCallEnd { name, .. } => ("ToolCallEnd", name.as_str()),
            AiBridgeChunk::Done { .. } => ("Done", ""),
        };
        let (text, truncated) = truncate_text(text);
        self.root
            .add_event(Event::new("mapped").with_properties(|| {
                [
                    ("kind", "mapped".into()),
                    ("variant", variant.into()),
                    ("text", text),
                    ("truncated", if truncated { "true" } else { "false" }.into()),
                ]
            }));
    }
}

fn truncate_text(text: &str) -> (String, bool) {
    let count = text.chars().count();
    if count <= PROVIDER_TRACE_TEXT_MAX {
        return (text.to_string(), false);
    }
    (text.chars().take(PROVIDER_TRACE_TEXT_MAX).collect(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_respects_max() {
        let long: String = "a".repeat(PROVIDER_TRACE_TEXT_MAX + 10);
        let (out, truncated) = truncate_text(&long);
        assert!(truncated);
        assert_eq!(out.chars().count(), PROVIDER_TRACE_TEXT_MAX);
    }

    #[test]
    fn inactive_start_returns_none() {
        set_provider_trace_active(false);
        assert!(ProviderRequestTrace::start("openai-responses", "m").is_none());
    }
}
