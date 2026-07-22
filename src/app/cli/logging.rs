//! Observability bootstrap — fastrace reporters + `log` file logger.
//!
//! File-only sinks never write stdout/stderr so TUI Inline + DSR stay intact.
//! Optional OTLP/HTTP (feature `otel` + `[otel]` config) is also file/network
//! only — never console.
//!
//! Local file activation (no CLI flag):
//! - `RUST_LOG` set → level filter from env
//! - else `XYLITOL_DEBUG=1` → `xylitol=debug,warn`
//! - else debug builds (`cfg(debug_assertions)`) → same default
//! - release with neither → off
//!
//! Provider timeline (`provider-trace.jsonl`) follows the same gate, or
//! `XYLITOL_PROVIDER_TRACE=1` alone in release.
//!
//! Remote OTLP is orthogonal: `[otel].exporter = otlp-http` (+ feature), default
//! `none`.

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use crate::infra::config::types::OtelConfig;
use crate::infra::observability::{FanoutReporter, FileTraceReporter};
use crate::infra::provider::trace::set_provider_trace_active;

const DEFAULT_FILTER: &str = "xylitol=debug,warn";

/// Install file-only log + fastrace reporters when requested.
///
/// `otel` controls optional remote OTLP (ignored / degraded when feature off or
/// misconfigured). Returns `Some(())` when any backend was installed.
pub fn init_logging(agent_dir: &Path, otel: &OtelConfig) -> Option<()> {
    let want_log = logging_requested();
    let want_provider = provider_trace_requested(want_log);
    let otel_reporter = crate::infra::observability::otel::install::try_build_otlp_reporter(otel);

    if !want_log && !want_provider && otel_reporter.is_none() {
        set_provider_trace_active(false);
        return None;
    }

    let log_dir = agent_dir.join("logs");
    std::fs::create_dir_all(&log_dir).ok()?;

    if want_log {
        let log_path = log_dir.join("xylitol.log");
        let file = open_append(&log_dir, &log_path)?;
        let filter = level_filter();
        match env_logger::Builder::new()
            .filter_level(log::LevelFilter::Warn)
            .parse_filters(&filter)
            .target(env_logger::Target::Pipe(Box::new(MutexWriter(Mutex::new(
                file,
            )))))
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} {:5} {} - {}",
                    buf.timestamp_millis(),
                    record.level(),
                    record.target(),
                    record.args()
                )
            })
            .try_init()
        {
            Ok(()) => {
                log::info!(
                    target: "xylitol::logging",
                    "logging enabled path={}",
                    log_path.display()
                );
            }
            Err(e) => {
                if let Some(mut f) = open_append(&log_dir, &log_path) {
                    let _ = writeln!(
                        f,
                        "xylitol::logging WARN env_logger init failed ({e}); \
                         level log sink inactive; provider-trace / otel may still run \
                         under the same logs/ dir"
                    );
                    let _ = f.flush();
                }
            }
        }
    }

    let mut fanout = FanoutReporter::new(Vec::new());
    // Span emission (provider_trace_active) is on when any sink needs spans:
    // local JSONL and/or OTLP. FileReporter alone still follows env/build gate.
    let mut emit_spans = false;

    if want_provider {
        let trace_path = log_dir.join("provider-trace.jsonl");
        match FileTraceReporter::open(trace_path.clone()) {
            Ok(reporter) => {
                fanout.push(reporter);
                emit_spans = true;
                log::info!(
                    target: "xylitol::logging",
                    "provider trace enabled path={}",
                    trace_path.display()
                );
            }
            Err(e) => {
                log::warn!(target: "xylitol::logging", "provider trace open failed: {e}");
            }
        }
    }

    if let Some(otel_r) = otel_reporter {
        fanout.push_box(otel_r);
        emit_spans = true;
        log::info!(
            target: "xylitol::otel",
            "OTLP export enabled exporter=otlp-http"
        );
    }

    set_provider_trace_active(emit_spans);

    if !fanout.is_empty() {
        fastrace::set_reporter(fanout, fastrace::collector::Config::default());
    }

    Some(())
}

/// Flush fastrace before process exit (file + OTLP batch).
pub fn flush_observability() {
    fastrace::flush();
}

fn logging_requested() -> bool {
    if std::env::var_os("RUST_LOG").is_some_and(|v| !v.is_empty()) {
        return true;
    }
    if std::env::var_os("XYLITOL_DEBUG").is_some_and(is_truthy) {
        return true;
    }
    cfg!(debug_assertions)
}

fn provider_trace_requested(logging_on: bool) -> bool {
    if std::env::var_os("XYLITOL_PROVIDER_TRACE").is_some_and(is_truthy) {
        return true;
    }
    if std::env::var_os("XYLITOL_PROVIDER_TRACE").is_some_and(is_falsey) {
        return false;
    }
    logging_on
}

fn level_filter() -> String {
    if let Ok(v) = std::env::var("RUST_LOG")
        && !v.is_empty()
    {
        return v;
    }
    DEFAULT_FILTER.to_string()
}

fn open_append(dir: &Path, path: &Path) -> Option<std::fs::File> {
    std::fs::create_dir_all(dir).ok()?;
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(path).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    let _ = file.flush();
    Some(file)
}

fn is_truthy(v: std::ffi::OsString) -> bool {
    let s = v.to_string_lossy();
    matches!(
        s.as_ref(),
        "1" | "true" | "yes" | "on" | "TRUE" | "YES" | "ON"
    )
}

fn is_falsey(v: std::ffi::OsString) -> bool {
    let s = v.to_string_lossy();
    matches!(
        s.as_ref(),
        "0" | "false" | "no" | "off" | "FALSE" | "NO" | "OFF"
    )
}

/// `env_logger::Target::Pipe` needs `Write`; Mutex around File is Sync.
struct MutexWriter(Mutex<std::fs::File>);

impl Write for MutexWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_append_creates_file_and_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("logs");
        let path = dir.join("xylitol.log");
        let _ = open_append(&dir, &path).expect("open_append succeeds");
        assert!(path.exists());
        let mut f = open_append(&dir, &path).unwrap();
        assert!(writeln!(f, "line1").is_ok());
        let mut f2 = open_append(&dir, &path).unwrap();
        assert!(writeln!(f2, "line2").is_ok());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("line1") && contents.contains("line2"));
    }

    #[test]
    fn is_truthy_matches_common_values() {
        assert!(is_truthy("1".into()));
        assert!(is_truthy("true".into()));
        assert!(!is_truthy("0".into()));
        assert!(is_falsey("0".into()));
    }
}
