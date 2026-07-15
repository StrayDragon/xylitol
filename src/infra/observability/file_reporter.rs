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
            let model = prop(&span.properties, "model").unwrap_or("");
            let trace_id = span.trace_id.to_string();
            let span_id = format!("{:016x}", span.span_id.0);

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
