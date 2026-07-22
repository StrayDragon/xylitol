//! Provider request timeline helpers (fastrace Events → provider-trace.jsonl).
//!
//! Runtime gate via [`provider_trace_active`]; when inactive, emit helpers are
//! no-ops and must not allocate large SSE strings.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use fastrace::prelude::*;

use crate::dto::{AiBridgeChunk, AiBridgeUsage};

/// Max Unicode scalars for `text` fields in provider-trace JSONL (c1000).
pub const PROVIDER_TRACE_TEXT_MAX: usize = 4096;

/// Hard cap for `observation_io = full` (protect OTLP batch size).
pub const OBSERVATION_IO_FULL_MAX: usize = 65_536;

static PROVIDER_TRACE_ACTIVE: AtomicBool = AtomicBool::new(false);
static OBSERVATION_IO_TIER: AtomicU8 = AtomicU8::new(0);

/// How much generation I/O to attach as Langfuse observation attributes (c1485).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ObservationIoTier {
    #[default]
    None = 0,
    Truncated = 1,
    Full = 2,
}

impl ObservationIoTier {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Truncated,
            2 => Self::Full,
            _ => Self::None,
        }
    }

    fn max_chars(self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Truncated => Some(PROVIDER_TRACE_TEXT_MAX),
            Self::Full => Some(OBSERVATION_IO_FULL_MAX),
        }
    }
}

/// Called from composition-root logging init when the FileReporter is installed.
pub fn set_provider_trace_active(active: bool) {
    PROVIDER_TRACE_ACTIVE.store(active, Ordering::Relaxed);
}

#[inline]
pub fn provider_trace_active() -> bool {
    PROVIDER_TRACE_ACTIVE.load(Ordering::Relaxed)
}

/// Called from composition root when `[otel].observation_io` is resolved.
pub fn set_observation_io_tier(tier: ObservationIoTier) {
    OBSERVATION_IO_TIER.store(tier as u8, Ordering::Relaxed);
}

#[inline]
pub fn observation_io_tier() -> ObservationIoTier {
    ObservationIoTier::from_u8(OBSERVATION_IO_TIER.load(Ordering::Relaxed))
}

/// Root span for one provider HTTP stream; drop (end of stream) reports to FileReporter.
pub struct ProviderRequestTrace {
    root: Span,
    #[allow(dead_code)]
    request_id: String,
    /// First request-shaped raw JSON (when I/O tier ≠ none).
    input_buf: Mutex<Option<String>>,
    /// Accumulated assistant text deltas (when I/O tier ≠ none).
    output_buf: Mutex<String>,
}

impl ProviderRequestTrace {
    pub fn start(api: &str, model: &str) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let root = Span::root("provider.request", SpanContext::random()).with_properties(|| {
            let mut props = vec![
                ("request_id".to_string(), request_id.clone()),
                ("api".to_string(), api.to_string()),
                ("model".to_string(), model.to_string()),
            ];
            props.extend(super::langfuse_generation_properties(model));
            props
        });
        Some(Self {
            root,
            request_id,
            input_buf: Mutex::new(None),
            output_buf: Mutex::new(String::new()),
        })
    }

    pub fn emit_raw(&self, event: &str, text: &str) {
        if !provider_trace_active() {
            return;
        }
        let (text, truncated) = truncate_text(text, PROVIDER_TRACE_TEXT_MAX);
        if observation_io_tier() != ObservationIoTier::None
            && is_request_body_event(event)
            && let Ok(mut slot) = self.input_buf.lock()
            && slot.is_none()
        {
            *slot = Some(text.clone());
        }
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
        let (text, truncated) = truncate_text(text, PROVIDER_TRACE_TEXT_MAX);
        self.root
            .add_event(Event::new("mapped").with_properties(|| {
                [
                    ("kind", "mapped".into()),
                    ("variant", variant.into()),
                    ("text", text),
                    ("truncated", if truncated { "true" } else { "false" }.into()),
                ]
            }));

        if let AiBridgeChunk::TextDelta(t) = chunk
            && observation_io_tier() != ObservationIoTier::None
            && let Ok(mut buf) = self.output_buf.lock()
        {
            append_capped(&mut buf, t, observation_io_tier().max_chars().unwrap_or(0));
        }

        if let AiBridgeChunk::Done { usage: Some(u), .. } = chunk {
            self.attach_usage(u);
            self.attach_observation_io();
        }
    }

    fn attach_usage(&self, u: &AiBridgeUsage) {
        self.root
            .add_property(|| ("gen_ai.usage.input_tokens", u.input.to_string()));
        self.root
            .add_property(|| ("gen_ai.usage.output_tokens", u.output.to_string()));
        let mut details = serde_json::json!({
            "input": u.input,
            "output": u.output,
            "total": u.input.saturating_add(u.output),
        });
        if u.cache_read > 0 {
            details["cache_read"] = serde_json::json!(u.cache_read);
        }
        if u.cache_write > 0 {
            details["cache_write"] = serde_json::json!(u.cache_write);
        }
        if u.cache_write_1h > 0 {
            details["cache_write_1h"] = serde_json::json!(u.cache_write_1h);
        }
        let details_s = details.to_string();
        self.root
            .add_property(|| ("langfuse.observation.usage_details", details_s));
    }

    fn attach_observation_io(&self) {
        let Some(max) = observation_io_tier().max_chars() else {
            return;
        };
        if let Ok(guard) = self.input_buf.lock()
            && let Some(ref input) = *guard
        {
            let (s, _) = truncate_text(input, max);
            self.root.add_property(|| ("langfuse.observation.input", s));
        }
        if let Ok(guard) = self.output_buf.lock()
            && !guard.is_empty()
        {
            let (s, _) = truncate_text(guard.as_str(), max);
            self.root
                .add_property(|| ("langfuse.observation.output", s));
        }
    }
}

fn is_request_body_event(event: &str) -> bool {
    matches!(
        event,
        "response.json" | "chat.completion.json" | "message.json"
    )
}

fn truncate_text(text: &str, max: usize) -> (String, bool) {
    let count = text.chars().count();
    if count <= max {
        return (text.to_string(), false);
    }
    (text.chars().take(max).collect(), true)
}

fn append_capped(buf: &mut String, chunk: &str, max: usize) {
    if max == 0 {
        return;
    }
    let have = buf.chars().count();
    if have >= max {
        return;
    }
    let room = max - have;
    let take: String = chunk.chars().take(room).collect();
    buf.push_str(&take);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn truncate_respects_max() {
        let long: String = "a".repeat(PROVIDER_TRACE_TEXT_MAX + 10);
        let (out, truncated) = truncate_text(&long, PROVIDER_TRACE_TEXT_MAX);
        assert!(truncated);
        assert_eq!(out.chars().count(), PROVIDER_TRACE_TEXT_MAX);
    }

    #[test]
    fn inactive_start_returns_none() {
        let _g = TEST_LOCK.lock().unwrap();
        set_provider_trace_active(false);
        set_observation_io_tier(ObservationIoTier::None);
        assert!(ProviderRequestTrace::start("openai-responses", "m").is_none());
    }

    #[test]
    fn usage_and_io_none_do_not_panic() {
        let _g = TEST_LOCK.lock().unwrap();
        set_provider_trace_active(true);
        set_observation_io_tier(ObservationIoTier::None);
        let t = ProviderRequestTrace::start("openai-responses", "m").expect("active");
        t.emit_mapped_chunk(&AiBridgeChunk::Done {
            finish_reason: crate::dto::AiBridgeStopReason::Stop,
            usage: Some(AiBridgeUsage {
                input: 3,
                output: 5,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: 0,
                total_tokens: 8,
                cost: None,
            }),
        });
        drop(t);
        set_provider_trace_active(false);
    }

    #[test]
    fn truncated_io_buffers_request_and_output() {
        let _g = TEST_LOCK.lock().unwrap();
        set_provider_trace_active(true);
        set_observation_io_tier(ObservationIoTier::Truncated);
        let t = ProviderRequestTrace::start("openai-completions", "m").expect("active");
        t.emit_raw(
            "chat.completion.json",
            r#"{"messages":[{"role":"user","content":"hi"}]}"#,
        );
        t.emit_mapped_chunk(&AiBridgeChunk::TextDelta("hello".into()));
        t.emit_mapped_chunk(&AiBridgeChunk::Done {
            finish_reason: crate::dto::AiBridgeStopReason::Stop,
            usage: Some(AiBridgeUsage {
                input: 1,
                output: 1,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: 0,
                total_tokens: 2,
                cost: None,
            }),
        });
        assert_eq!(
            t.input_buf.lock().unwrap().as_deref(),
            Some(r#"{"messages":[{"role":"user","content":"hi"}]}"#)
        );
        assert_eq!(t.output_buf.lock().unwrap().as_str(), "hello");
        drop(t);
        set_observation_io_tier(ObservationIoTier::None);
        set_provider_trace_active(false);
    }
}
