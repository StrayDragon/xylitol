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
static TOOL_OBSERVATION_IO_TIER: AtomicU8 = AtomicU8::new(0);

/// How much generation / turn I/O to attach as Langfuse observation attributes (c1485 / c1555).
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

    /// Unicode-scalar cap for observation I/O at this tier (`None` → no I/O).
    pub fn max_chars(self) -> Option<usize> {
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

/// Called from composition root when `[otel].tool_observation_io` is resolved (c1550).
pub fn set_tool_observation_io_tier(tier: ObservationIoTier) {
    TOOL_OBSERVATION_IO_TIER.store(tier as u8, Ordering::Relaxed);
}

#[inline]
pub fn tool_observation_io_tier() -> ObservationIoTier {
    ObservationIoTier::from_u8(TOOL_OBSERVATION_IO_TIER.load(Ordering::Relaxed))
}

/// Truncate for Langfuse observation I/O (shared by generation / tool / turn).
pub fn truncate_observation_text(text: &str, max: usize) -> (String, bool) {
    truncate_text(text, max)
}

/// How a generation span closed (c1590).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationFinishKind {
    /// Normal stream end (`Done`, with or without usage).
    Completed,
    /// Abort / drop without successful `Done` — ERROR + `aborted`.
    Aborted,
}

/// Span for one provider HTTP stream (`llm.request`); drop reports to FileReporter.
pub struct ProviderRequestTrace {
    root: Span,
    #[allow(dead_code)]
    request_id: String,
    /// First request-shaped JSON (when I/O tier ≠ none).
    input_buf: Mutex<Option<String>>,
    /// Accumulated assistant text deltas (when I/O tier ≠ none).
    output_buf: Mutex<String>,
    finalized: AtomicBool,
}

impl ProviderRequestTrace {
    /// Start as an independent root (`None` parent → random context).
    /// Prefer [`Self::start_with_parent`] when an iteration / compaction parent is known.
    pub fn start(api: &str, model: &str) -> Option<Self> {
        Self::start_with_parent(api, model, None)
    }

    pub fn start_with_parent(api: &str, model: &str, parent: Option<SpanContext>) -> Option<Self> {
        if !provider_trace_active() {
            return None;
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let parent_ctx = parent.unwrap_or_else(SpanContext::random);
        let root = Span::root("llm.request", parent_ctx).with_properties(|| {
            // `api` / `request_id` are xylitol-local; model goes only via
            // `langfuse.observation.model.name` (see langfuse_generation_properties).
            let mut props = vec![
                ("request_id".to_string(), request_id.clone()),
                ("api".to_string(), api.to_string()),
            ];
            props.extend(super::langfuse_generation_properties(model));
            props
        });
        Some(Self {
            root,
            request_id,
            input_buf: Mutex::new(None),
            output_buf: Mutex::new(String::new()),
            finalized: AtomicBool::new(false),
        })
    }

    /// Capture assembled request JSON before HTTP (c1590). Idempotent first-wins.
    pub fn capture_request_input(&self, body_json: &str) {
        if observation_io_tier() == ObservationIoTier::None {
            return;
        }
        let Some(max) = observation_io_tier().max_chars() else {
            return;
        };
        if let Ok(mut slot) = self.input_buf.lock()
            && slot.is_none()
        {
            let (s, _) = truncate_text(body_json, max);
            *slot = Some(s);
        }
    }

    pub fn emit_raw(&self, event: &str, text: &str) {
        if !provider_trace_active() {
            return;
        }
        let (text, truncated) = truncate_text(text, PROVIDER_TRACE_TEXT_MAX);
        // Legacy fallback: still accept request-shaped raw event names if capture missed.
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

        if let AiBridgeChunk::Done { usage, .. } = chunk {
            if let Some(u) = usage {
                self.attach_usage(u);
            }
            self.finalize(GenerationFinishKind::Completed);
        }
    }

    /// Idempotent close: attach I/O; on [`GenerationFinishKind::Aborted`] mark ERROR.
    pub fn finalize(&self, kind: GenerationFinishKind) {
        if self
            .finalized
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        self.attach_observation_io();
        if kind == GenerationFinishKind::Aborted {
            self.root
                .add_property(|| ("langfuse.observation.level", "ERROR".to_string()));
            self.root
                .add_property(|| ("langfuse.observation.status_message", "aborted".to_string()));
        }
    }

    fn attach_usage(&self, u: &AiBridgeUsage) {
        // Single usage channel: Langfuse `usage_details` (verbatim / exclusive
        // buckets, incl. optional cache). Do not also emit `gen_ai.usage.*` —
        // same mapped field with inclusive/normalize semantics that fight cache.
        let mut details = serde_json::json!({
            "input": u.input,
            "output": u.output,
            "total": u.input.saturating_add(u.output),
        });
        // Honesty (c1885): only write cache_read under Tokens(n); never fake 0
        // for NotReported / NotApplicable.
        if let crate::dto::PromptCacheRead::Tokens(n) = u.prompt_cache_read {
            details["cache_read"] = serde_json::json!(n);
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
        let status = u.prompt_cache_read.as_status_str().to_string();
        self.root
            .add_property(|| ("xylitol.prompt_cache_read", status));
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

impl Drop for ProviderRequestTrace {
    fn drop(&mut self) {
        // Mid-stream abort / early close: flush partial I/O + ERROR (c1590).
        self.finalize(GenerationFinishKind::Aborted);
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
    use fastrace::collector::{Config, Reporter, SpanRecord};
    use std::sync::{Arc, Mutex};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    struct CollectingReporter(Arc<Mutex<Vec<SpanRecord>>>);

    impl Reporter for CollectingReporter {
        fn report(&mut self, spans: Vec<SpanRecord>) {
            self.0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .extend(spans);
        }
    }

    fn prop<'a>(span: &'a SpanRecord, key: &str) -> Option<&'a str> {
        span.properties
            .iter()
            .find(|(k, _)| k.as_ref() == key)
            .map(|(_, v)| v.as_ref())
    }

    fn take_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn latest_llm(spans: &[SpanRecord]) -> &SpanRecord {
        spans
            .iter()
            .rev()
            .find(|s| s.name == "llm.request")
            .expect("llm.request")
    }

    #[test]
    fn truncate_respects_max() {
        let long: String = "a".repeat(PROVIDER_TRACE_TEXT_MAX + 10);
        let (out, truncated) = truncate_text(&long, PROVIDER_TRACE_TEXT_MAX);
        assert!(truncated);
        assert_eq!(out.chars().count(), PROVIDER_TRACE_TEXT_MAX);
    }

    #[test]
    fn inactive_start_returns_none() {
        let _g = take_lock();
        set_provider_trace_active(false);
        set_observation_io_tier(ObservationIoTier::None);
        assert!(ProviderRequestTrace::start("openai-responses", "m").is_none());
    }

    #[test]
    fn usage_and_io_none_do_not_panic() {
        let _g = take_lock();
        set_provider_trace_active(true);
        set_observation_io_tier(ObservationIoTier::None);
        let t = ProviderRequestTrace::start("openai-responses", "m").expect("active");
        t.emit_mapped_chunk(&AiBridgeChunk::Done {
            finish_reason: crate::dto::AiBridgeStopReason::Stop,
            usage: Some(AiBridgeUsage {
                input: 3,
                output: 5,
                total_tokens: 8,
                ..Default::default()
            }),
        });
        drop(t);
        set_provider_trace_active(false);
    }

    #[test]
    fn capture_request_input_and_done_flush_io() {
        let _g = take_lock();
        set_provider_trace_active(true);
        set_observation_io_tier(ObservationIoTier::Truncated);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let t = ProviderRequestTrace::start("openai-responses", "m").expect("active");
            t.capture_request_input(r#"{"model":"m","input":[{"role":"user","content":"hi"}]}"#);
            t.emit_mapped_chunk(&AiBridgeChunk::TextDelta("hello".into()));
            t.emit_mapped_chunk(&AiBridgeChunk::Done {
                finish_reason: crate::dto::AiBridgeStopReason::Stop,
                usage: Some(AiBridgeUsage {
                    input: 1,
                    output: 1,
                    total_tokens: 2,
                    ..Default::default()
                }),
            });
            drop(t);
        }
        fastrace::flush();
        set_observation_io_tier(ObservationIoTier::None);
        set_provider_trace_active(false);

        let spans = records.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let llm = latest_llm(&spans);
        let input = prop(llm, "langfuse.observation.input").expect("input");
        assert!(input.contains("\"role\":\"user\""), "{input}");
        assert_eq!(prop(llm, "langfuse.observation.output"), Some("hello"));
        assert!(prop(llm, "langfuse.observation.usage_details").is_some());
        assert!(prop(llm, "langfuse.observation.level").is_none());
    }

    #[test]
    fn attach_usage_emits_tri_state_without_fake_cache_read() {
        let _g = take_lock();
        set_provider_trace_active(true);
        set_observation_io_tier(ObservationIoTier::None);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let t = ProviderRequestTrace::start("openai-responses", "m").expect("active");
            let usage = AiBridgeUsage {
                input: 10,
                output: 2,
                total_tokens: 12,
                ..Default::default()
            }
            .with_prompt_cache_read(crate::dto::PromptCacheRead::NotReported);
            t.emit_mapped_chunk(&AiBridgeChunk::Done {
                finish_reason: crate::dto::AiBridgeStopReason::Stop,
                usage: Some(usage),
            });
            drop(t);
        }
        fastrace::flush();
        set_provider_trace_active(false);

        let spans = records.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let llm = latest_llm(&spans);
        assert_eq!(prop(llm, "xylitol.prompt_cache_read"), Some("not_reported"));
        let details = prop(llm, "langfuse.observation.usage_details").expect("details");
        assert!(
            !details.contains("cache_read"),
            "must not forge cache_read for NotReported: {details}"
        );
    }

    #[test]
    fn attach_usage_writes_cache_read_for_tokens() {
        let _g = take_lock();
        set_provider_trace_active(true);
        set_observation_io_tier(ObservationIoTier::None);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let t = ProviderRequestTrace::start("openai-responses", "m").expect("active");
            let usage = AiBridgeUsage {
                input: 10,
                output: 2,
                total_tokens: 12,
                ..Default::default()
            }
            .with_prompt_cache_read(crate::dto::PromptCacheRead::Tokens(7));
            t.emit_mapped_chunk(&AiBridgeChunk::Done {
                finish_reason: crate::dto::AiBridgeStopReason::Stop,
                usage: Some(usage),
            });
            drop(t);
        }
        fastrace::flush();
        set_provider_trace_active(false);

        let spans = records.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let llm = latest_llm(&spans);
        assert_eq!(prop(llm, "xylitol.prompt_cache_read"), Some("tokens"));
        let details = prop(llm, "langfuse.observation.usage_details").expect("details");
        assert!(
            details.contains("\"cache_read\":7"),
            "expected Tokens(7) in usage_details: {details}"
        );
    }

    #[test]
    fn abort_drop_flushes_partial_and_marks_error() {
        let _g = take_lock();
        set_provider_trace_active(true);
        set_observation_io_tier(ObservationIoTier::Truncated);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let t = ProviderRequestTrace::start("openai-responses", "m").expect("active");
            t.capture_request_input(r#"{"model":"m","stream":true}"#);
            t.emit_mapped_chunk(&AiBridgeChunk::TextDelta("partial…".into()));
            // No Done — Drop ⇒ aborted finalize.
            drop(t);
        }
        fastrace::flush();
        set_observation_io_tier(ObservationIoTier::None);
        set_provider_trace_active(false);

        let spans = records.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let llm = latest_llm(&spans);
        assert!(
            prop(llm, "langfuse.observation.input").is_some_and(|s| s.contains("stream")),
            "{:?}",
            llm.properties
        );
        assert_eq!(prop(llm, "langfuse.observation.output"), Some("partial…"));
        assert_eq!(prop(llm, "langfuse.observation.level"), Some("ERROR"));
        assert_eq!(
            prop(llm, "langfuse.observation.status_message"),
            Some("aborted")
        );
        assert!(
            prop(llm, "langfuse.observation.usage_details").is_none(),
            "must not forge usage on abort"
        );
    }

    #[test]
    fn done_without_usage_still_flushes_io() {
        let _g = take_lock();
        set_provider_trace_active(true);
        set_observation_io_tier(ObservationIoTier::Truncated);
        let records = Arc::new(Mutex::new(Vec::new()));
        fastrace::set_reporter(CollectingReporter(Arc::clone(&records)), Config::default());

        {
            let t = ProviderRequestTrace::start("openai-completions", "m").expect("active");
            t.capture_request_input(r#"{"messages":[],"marker":"done-no-usage"}"#);
            t.emit_mapped_chunk(&AiBridgeChunk::TextDelta("x".into()));
            t.emit_mapped_chunk(&AiBridgeChunk::Done {
                finish_reason: crate::dto::AiBridgeStopReason::Stop,
                usage: None,
            });
            drop(t);
        }
        fastrace::flush();
        set_observation_io_tier(ObservationIoTier::None);
        set_provider_trace_active(false);

        let spans = records.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let llm = spans
            .iter()
            .rev()
            .find(|s| {
                s.name == "llm.request"
                    && prop(s, "langfuse.observation.input")
                        .is_some_and(|i| i.contains("done-no-usage"))
            })
            .expect("llm");
        assert_eq!(prop(llm, "langfuse.observation.output"), Some("x"));
        assert!(prop(llm, "langfuse.observation.usage_details").is_none());
        assert!(prop(llm, "langfuse.observation.level").is_none());
    }

    #[test]
    fn truncated_io_buffers_request_and_output() {
        let _g = take_lock();
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
                total_tokens: 2,
                ..Default::default()
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
