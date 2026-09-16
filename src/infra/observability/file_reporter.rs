//! File-only fastrace [`Reporter`] — never writes stdout/stderr (TUI-safe).
//!
//! Flattens span events into `provider-trace.jsonl` lines matching
//! `xylitol.provider_trace.v1` (see c1000 design.md).

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use fastrace::collector::{Reporter, SpanRecord};
use serde_json::{Map, Value};

/// Schema id written on every JSONL line (bump only on breaking changes).
pub const PROVIDER_TRACE_SCHEMA: &str = "xylitol.provider_trace.v1";

pub struct FileTraceReporter {
    file: Mutex<File>,
}

impl FileTraceReporter {
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut opts = std::fs::OpenOptions::new();
        opts.create(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let file = opts.open(&path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(Self {
            file: Mutex::new(file),
        })
    }
}

impl Reporter for FileTraceReporter {
    fn report(&mut self, spans: Vec<SpanRecord>) {
        let Ok(mut file) = self.file.lock() else {
            return;
        };
        for span in spans {
            let request_id = prop(&span.properties, "request_id").unwrap_or("");
            let api = prop(&span.properties, "api").unwrap_or("");
            // Model lives on Langfuse key after attribute slim; keep bare `model` fallback.
            let model = prop(&span.properties, "model")
                .or_else(|| prop(&span.properties, "langfuse.observation.model.name"))
                .unwrap_or("");
            let trace_id = span.trace_id.to_string();
            let span_id = format!("{:016x}", span.span_id.0);

            // Span-level correlation for lifecycle (agent.turn / iteration / tool.execute).
            let span_turn_id = prop(&span.properties, "turn_id");
            let span_tool_name = prop(&span.properties, "tool_name");
            let span_tool_id = prop(&span.properties, "tool_id");
            let span_compaction_requested =
                prop(&span.properties, "xylitol.compaction.requested_model");
            let span_compaction_actual = prop(&span.properties, "xylitol.compaction.actual_model");
            let span_compaction_fallback =
                prop(&span.properties, "xylitol.compaction.model_fallback");
            let span_compaction_thinking_rejected =
                prop(&span.properties, "xylitol.compaction.thinking_rejected");

            for ev in &span.events {
                let kind = prop(&ev.properties, "kind").unwrap_or(ev.name.as_ref());
                let mut obj = Map::new();
                obj.insert("schema".into(), Value::String(PROVIDER_TRACE_SCHEMA.into()));
                obj.insert(
                    "ts_unix_ns".into(),
                    Value::Number(ev.timestamp_unix_ns.into()),
                );
                obj.insert("trace_id".into(), Value::String(trace_id.clone()));
                obj.insert("span_id".into(), Value::String(span_id.clone()));
                obj.insert("request_id".into(), Value::String(request_id.into()));
                obj.insert("kind".into(), Value::String(kind.into()));
                obj.insert("api".into(), Value::String(api.into()));
                obj.insert("model".into(), Value::String(model.into()));
                obj.insert("ext".into(), Value::Object(Map::new()));

                if let Some(event) = prop(&ev.properties, "event") {
                    obj.insert("event".into(), Value::String(event.into()));
                }
                if let Some(variant) = prop(&ev.properties, "variant") {
                    obj.insert("variant".into(), Value::String(variant.into()));
                }
                if let Some(text) = prop(&ev.properties, "text") {
                    obj.insert("text".into(), Value::String(text.into()));
                }
                // Lifecycle / low-freq span fields (c1265) — must not be dropped.
                for key in [
                    "name",
                    "phase",
                    "turn_id",
                    "turn_index",
                    "tool_name",
                    "tool_id",
                    "span_role",
                    // token.estimate (c1420 follow-up); `backend` kept for old lines
                    "backend",
                    "provenance",
                    "tokens",
                    // react.error / tool.error diagnostics (record_xy_error /
                    // record_tool_error) — without these the error rows carry no
                    // explanation beyond correlation ids.
                    "error.kind",
                    "where",
                    "message",
                    "xylitol.compaction.requested_model",
                    "xylitol.compaction.actual_model",
                    "xylitol.compaction.model_fallback",
                    "xylitol.compaction.thinking_rejected",
                ] {
                    if let Some(v) = prop(&ev.properties, key) {
                        obj.insert(key.into(), Value::String(v.into()));
                    }
                }
                for (key, value) in [
                    (
                        "xylitol.compaction.requested_model",
                        span_compaction_requested,
                    ),
                    ("xylitol.compaction.actual_model", span_compaction_actual),
                    (
                        "xylitol.compaction.model_fallback",
                        span_compaction_fallback,
                    ),
                    (
                        "xylitol.compaction.thinking_rejected",
                        span_compaction_thinking_rejected,
                    ),
                ] {
                    if !obj.contains_key(key)
                        && let Some(v) = value
                    {
                        obj.insert(key.into(), Value::String(v.into()));
                    }
                }
                if !obj.contains_key("turn_id")
                    && let Some(tid) = span_turn_id
                {
                    obj.insert("turn_id".into(), Value::String(tid.into()));
                }
                if !obj.contains_key("tool_name")
                    && let Some(name) = span_tool_name
                {
                    obj.insert("tool_name".into(), Value::String(name.into()));
                }
                if !obj.contains_key("tool_id")
                    && let Some(id) = span_tool_id
                {
                    obj.insert("tool_id".into(), Value::String(id.into()));
                }
                let truncated = prop(&ev.properties, "truncated").is_some_and(|v| v == "true");
                obj.insert("truncated".into(), Value::Bool(truncated));

                if let Ok(line) = serde_json::to_string(&Value::Object(obj)) {
                    let _ = writeln!(file, "{line}");
                }
            }
        }
        let _ = file.flush();
    }
}

fn prop<'a>(
    props: &'a [(
        std::borrow::Cow<'static, str>,
        std::borrow::Cow<'static, str>,
    )],
    key: &str,
) -> Option<&'a str> {
    props
        .iter()
        .find(|(k, _)| k.as_ref() == key)
        .map(|(_, v)| v.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastrace::collector::{EventRecord, SpanId, SpanRecord, TraceId};
    use std::borrow::Cow;

    /// `react.error` / `tool.error` events must keep their diagnostic fields
    /// (`error.kind` / `where` / `message`) in the JSONL row — without them an
    /// error line is unexplainable beyond correlation ids.
    #[test]
    fn error_events_keep_diagnostic_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("provider-trace.jsonl");
        let mut reporter = FileTraceReporter::open(path.clone()).unwrap();
        reporter.report(vec![SpanRecord {
            trace_id: TraceId(1),
            span_id: SpanId(2),
            parent_id: SpanId(0),
            begin_time_unix_ns: 1,
            duration_ns: 1,
            name: Cow::Borrowed("react.error"),
            properties: vec![(Cow::Borrowed("turn_id"), Cow::Borrowed("t-1"))],
            events: vec![EventRecord {
                name: Cow::Borrowed("error"),
                timestamp_unix_ns: 2,
                properties: vec![
                    (Cow::Borrowed("error.kind"), Cow::Borrowed("Provider")),
                    (Cow::Borrowed("where"), Cow::Borrowed("react.run")),
                    (Cow::Borrowed("message"), Cow::Borrowed("boom")),
                ],
            }],
            links: vec![],
        }]);
        let line = std::fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(v["kind"], "error");
        assert_eq!(v["error.kind"], "Provider");
        assert_eq!(v["where"], "react.run");
        assert_eq!(v["message"], "boom");
        // Span-level correlation fallback still applies.
        assert_eq!(v["turn_id"], "t-1");
    }
}
