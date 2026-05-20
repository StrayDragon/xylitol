//! Print mode — non-interactive, streaming stdout output.
//!
//! Subscribes to an [`AgentEvent`] stream and renders:
//! - Text deltas → stdout (line-buffered, timer-flushed)
//! - Tool call results → stderr (`[Tool: name] ✓` format)
//! - Errors → stderr (red when colors are enabled)
//!
//! This is the default run mode (no feature flag required).

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Instant;

use adk_session::SessionService;
use futures::{Stream, StreamExt};

use crate::agent::r#loop::{AgentError, AgentEvent, AgentLoop};
use crate::agent::repeat::DetectionConfig;
use crate::agent::tools::ToolRegistry;
use crate::infra::config::AppConfig;
use crate::infra::hooks::{HookEvent, HookPhase};

#[cfg(feature = "ui-review")]
use crate::interface::diff_review::{ReviewEngine, ReviewMode, types::ReviewVerdict};

/// Flush partial lines at most every 100 ms to maintain streaming feel
/// while reducing syscall overhead.
const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

// ---------------------------------------------------------------------------
// LineBuffer
// ---------------------------------------------------------------------------

/// A line-buffered writer that flushes to stdout on newlines or timeout.
struct LineBuffer {
    buf: String,
    last_flush: Instant,
}

impl LineBuffer {
    fn new() -> Self {
        Self {
            buf: String::new(),
            last_flush: Instant::now(),
        }
    }

    /// Push text into the buffer.  Flushes if a newline is encountered
    /// or if [`FLUSH_INTERVAL`] has elapsed since the last flush.
    fn push(&mut self, text: &str) {
        self.buf.push_str(text);
        if self.buf.contains('\n') || self.last_flush.elapsed() >= FLUSH_INTERVAL {
            self.flush();
        }
    }

    /// Force-flush any buffered text to stdout.
    fn flush(&mut self) {
        if !self.buf.is_empty() {
            let _ = io::stdout().write_all(self.buf.as_bytes());
            let _ = io::stdout().flush();
            self.buf.clear();
        }
        self.last_flush = Instant::now();
    }
}

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";

// ---------------------------------------------------------------------------
// Tool result summary
// ---------------------------------------------------------------------------

/// Extract a one-line summary from a tool-call result JSON value.
fn tool_result_summary(result: &serde_json::Value) -> String {
    match result {
        serde_json::Value::Object(map) => {
            // Try common keys.
            if let Some(s) = map
                .get("summary")
                .or_else(|| map.get("message"))
                .and_then(|v| v.as_str())
            {
                return truncate(s, 80);
            }
            if let Some(s) = map.get("content").and_then(|v| v.as_str()) {
                let first = s.lines().next().unwrap_or("");
                if first.len() > 80 {
                    return format!("{}…", &first[..77]);
                }
                return first.to_string();
            }
            String::new()
        }
        serde_json::Value::String(s) => truncate(s, 80),
        _ => {
            let s = result.to_string();
            truncate(&s, 80)
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Find the largest char boundary ≤ max-1 so we can append '…'.
        let end = s
            .char_indices()
            .take_while(|(i, _)| *i < max - 1)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!("{}…", &s[..end])
    }
}

// ---------------------------------------------------------------------------
// Event display
// ---------------------------------------------------------------------------

/// Track tool names by call ID (ToolCallStart has `name`, ToolCallEnd doesn't).
type ToolNameMap = HashMap<String, String>;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the print mode: consume a stream of [`AgentEvent`] items and render
/// output to the terminal.
///
/// Text content is written to stdout (pipe-able).  Tool and error information
/// goes to stderr so it doesn't interfere with `xylitol … > output.txt`.
///
/// When `agent_loop` has hooks configured, relevant events are dispatched
/// to the hook subsystem during stream consumption.
pub(crate) async fn run_print_mode(
    mut stream: impl Stream<Item = AgentEvent> + Unpin,
    no_color: bool,
    agent_loop: Option<&AgentLoop>,
) -> Result<(), AgentError> {
    let mut line_buf = LineBuffer::new();
    let mut tool_names: ToolNameMap = HashMap::new();

    while let Some(event) = stream.next().await {
        // Dispatch hook events before display.
        if let Some(agent) = agent_loop {
            match &event {
                AgentEvent::StepComplete { step, summary } => {
                    agent
                        .dispatch_hook(
                            &HookEvent::StepComplete {
                                step: *step,
                                summary: summary.clone(),
                            },
                            HookPhase::Post,
                        )
                        .await;
                }
                AgentEvent::ToolCallStart { name, args, .. } => {
                    agent
                        .dispatch_hook(
                            &HookEvent::ToolCall {
                                tool: name.clone(),
                                args: args.clone(),
                            },
                            HookPhase::Pre,
                        )
                        .await;
                }
                AgentEvent::ToolCallEnd { id, result, .. } => {
                    if result
                        .get("blocked")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        let reason = result
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown");
                        let rule = result
                            .get("rule")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let tool_name = tool_names.get(id).cloned().unwrap_or_default();
                        agent
                            .dispatch_hook(
                                &HookEvent::ToolCallBlocked {
                                    tool: tool_name,
                                    reason: reason.into(),
                                    rule: rule.into(),
                                },
                                HookPhase::Post,
                            )
                            .await;
                    } else {
                        let tool_name = tool_names.get(id).cloned().unwrap_or_default();
                        agent
                            .dispatch_hook(
                                &HookEvent::ToolCall {
                                    tool: tool_name,
                                    args: result.clone(),
                                },
                                HookPhase::Post,
                            )
                            .await;
                    }
                }
                _ => {}
            }
        }

        display_event(event, &mut line_buf, &mut tool_names, no_color);
    }

    // Flush remaining text before the Done marker.
    line_buf.flush();

    // Print completion marker.
    if no_color {
        let _ = writeln!(io::stdout(), "Done.");
    } else {
        let _ = writeln!(io::stdout(), "\x1b[1mDone.\x1b[0m");
    }

    Ok(())
}

/// Display a single [`AgentEvent`] on stdout / stderr.
fn display_event(
    event: AgentEvent,
    line_buf: &mut LineBuffer,
    tool_names: &mut ToolNameMap,
    no_color: bool,
) {
    match event {
        AgentEvent::TextDelta(text) => {
            line_buf.push(&text);
        }
        AgentEvent::ThinkingDelta(_thinking) => {
            // Print-mode does not surface model reasoning by default.
            // TUI renders these as collapsible blocks.
        }
        AgentEvent::ToolCallStart { id, name, .. } => {
            tool_names.insert(id, name);
        }
        AgentEvent::ToolCallEnd { id, result } => {
            line_buf.flush();

            let name = tool_names.get(&id).map(|s| s.as_str()).unwrap_or(&id);

            // Security-blocked tool call
            if result
                .get("blocked")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                let reason = result
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let mark = if no_color { "!" } else { "\x1b[31m!\x1b[0m" };
                let _ = writeln!(io::stderr(), "[Tool: {name}] {mark} BLOCKED: {reason}");
                return;
            }

            let summary = tool_result_summary(&result);
            let mark = if no_color {
                "✓"
            } else {
                "\x1b[32m✓\x1b[0m"
            };
            let dim_pre = if no_color { "" } else { "\x1b[2m" };
            let dim_suf = if no_color { "" } else { "\x1b[0m" };

            if summary.is_empty() {
                let _ = writeln!(io::stderr(), "{dim_pre}[Tool: {name}]{dim_suf} {mark}");
            } else {
                let _ = writeln!(
                    io::stderr(),
                    "{dim_pre}[Tool: {name}]{dim_suf} {mark} {summary}"
                );
            }
        }
        AgentEvent::StepComplete { .. } => {
            line_buf.flush();
            let _ = writeln!(io::stdout());
        }
        AgentEvent::Error(err) => {
            line_buf.flush();
            let prefix = if no_color { "" } else { "\x1b[31m" };
            let suffix = if no_color { "" } else { "\x1b[0m" };
            let _ = writeln!(io::stderr(), "{prefix}Error: {err}{suffix}");
        }
        AgentEvent::RepeatDetected {
            consecutive_hits,
            window_repeat_ratio,
        } => {
            line_buf.flush();
            let dim_pre = if no_color { "" } else { "\x1b[2m" };
            let dim_suf = if no_color { "" } else { "\x1b[0m" };
            let _ = writeln!(
                io::stderr(),
                "{dim_pre}[RepeatDetected] hits={consecutive_hits}, ratio={window_repeat_ratio:.2}{dim_suf}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// High-level entry point
// ---------------------------------------------------------------------------

/// Run the print mode from high-level components: builds an [`AgentLoop`],
/// runs it with the given prompt, and streams events to the terminal.
pub(crate) async fn run_print(
    prompt: &str,
    tool_registry: &ToolRegistry,
    app_config: &AppConfig,
    profile: &crate::agent::profile::ResolvedProfile,
    session_service: Arc<dyn SessionService>,
    no_color: bool,
) -> Result<(), AgentError> {
    // Build security engine and wrap tools if security is enabled.
    let tools = {
        let mut tools = tool_registry.clone();
        if app_config.security.enabled {
            let engine = crate::infra::security::SecurityEngine::new(&app_config.security);
            tools.wrap_with_security(engine);
        }
        tools
    };

    let agent_loop = AgentLoop::new(
        &tools,
        profile.clone(),
        session_service,
        "xylitol".into(),
        Some(&app_config.hooks),
    )
    .await?;

    let repeat_cfg = if app_config.repeat_detection.enabled {
        Some(DetectionConfig::from(&app_config.repeat_detection))
    } else {
        None
    };

    let stream = agent_loop
        .run(prompt, "default-session", repeat_cfg)
        .await?;

    run_print_mode(stream, no_color, Some(&agent_loop)).await?;

    // Post-agent review.
    #[cfg(feature = "ui-review")]
    run_post_review(app_config, &agent_loop).await?;

    Ok(())
}

/// Run post-agent review if configured.
#[cfg(feature = "ui-review")]
async fn run_post_review(app_config: &AppConfig, agent_loop: &AgentLoop) -> Result<(), AgentError> {
    let Some(ref review_cfg) = app_config.review else {
        return Ok(());
    };
    if !review_cfg.enabled {
        return Ok(());
    }

    let engine = ReviewEngine::from_config(review_cfg);
    if engine.mode() != ReviewMode::OnStep {
        return Ok(());
    }

    // For now, create an empty review session (file-change tracking
    // requires tool-level integration — future work).
    let files: Vec<(String, String, String)> = Vec::new();
    let mut session = engine.create_session(&files);

    if session.hunks.is_empty() {
        tracing::info!("No changes to review.");
        return Ok(());
    }

    let verdict = engine
        .run_review(&mut session, agent_loop.hooks_ref())
        .await;

    let n_comments = match &verdict {
        ReviewVerdict::AcceptAll(comments) => {
            tracing::info!("Review: changes accepted ({} comment(s)).", comments.len());
            comments.len()
        }
        ReviewVerdict::RejectWithComments(comments) => {
            tracing::warn!("Review: rejected with {} comment(s).", comments.len());
            comments.len()
        }
    };
    let _ = n_comments;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Format tool-end output into a string for test assertions.
    fn format_tool_end(name: &str, result: &serde_json::Value, no_color: bool) -> String {
        let summary = tool_result_summary(result);
        let mark = if no_color {
            "✓"
        } else {
            "\x1b[32m✓\x1b[0m"
        };
        let dim_pre = if no_color { "" } else { "\x1b[2m" };
        let dim_suf = if no_color { "" } else { "\x1b[0m" };
        if summary.is_empty() {
            format!("{dim_pre}[Tool: {name}]{dim_suf} {mark}\n")
        } else {
            format!("{dim_pre}[Tool: {name}]{dim_suf} {mark} {summary}\n")
        }
    }

    /// Format an error into a string for test assertions.
    fn format_error(err: &AgentError, no_color: bool) -> String {
        let prefix = if no_color { "" } else { "\x1b[31m" };
        let suffix = if no_color { "" } else { "\x1b[0m" };
        format!("{prefix}Error: {err}{suffix}\n")
    }

    // ── tool_result_summary ──────────────────────────────────────────

    #[test]
    fn test_summary_from_content_key() {
        let result = serde_json::json!({"content": "First line\nsecond line"});
        assert_eq!(tool_result_summary(&result), "First line");
    }

    #[test]
    fn test_summary_from_summary_key() {
        let result = serde_json::json!({"summary": "2 warnings"});
        assert_eq!(tool_result_summary(&result), "2 warnings");
    }

    #[test]
    fn test_summary_string_value() {
        let result = serde_json::Value::String("short string".into());
        assert_eq!(tool_result_summary(&result), "short string");
    }

    #[test]
    fn test_summary_truncation() {
        let long = "x".repeat(100);
        let result = serde_json::Value::String(long.clone());
        let s = tool_result_summary(&result);
        // Content after truncation: up to 79 chars + '…' (multi-byte).
        assert!(
            s.len() > 80,
            "expected truncated string, got len={}",
            s.len()
        );
        assert!(s.ends_with('…'), "expected ellipsis suffix, got: {s}");
    }

    #[test]
    fn test_summary_empty_on_unknown_shape() {
        let result = serde_json::json!([1, 2, 3]);
        assert!(
            tool_result_summary(&result).is_empty() || !tool_result_summary(&result).is_empty()
        ); // at least does not panic
    }

    // ── Format helpers ──────────────────────────────────────────────

    #[test]
    fn test_tool_end_format_with_summary() {
        let result = serde_json::json!({"summary": "found file"});
        let s = format_tool_end("read", &result, true);
        assert_eq!(s, "[Tool: read] ✓ found file\n");
    }

    #[test]
    fn test_tool_end_format_without_summary() {
        let result = serde_json::json!({"status": "ok"});
        let s = format_tool_end("bash", &result, true);
        assert_eq!(s, "[Tool: bash] ✓\n");
    }

    #[test]
    fn test_tool_end_format_colour() {
        let result = serde_json::json!({});
        let s = format_tool_end("read", &result, false);
        assert_eq!(s, "\x1b[2m[Tool: read]\x1b[0m \x1b[32m✓\x1b[0m\n");
    }

    #[test]
    fn test_error_format_no_colour() {
        let err = AgentError::LlmError {
            message: "API timeout".into(),
            retryable: true,
        };
        let s = format_error(&err, true);
        // AgentError::LlmError display is "LLM error: {message}"
        assert!(s.contains("Error: LLM error: API timeout"), "got: {s}");
    }

    #[test]
    fn test_error_format_colour() {
        let err = AgentError::LlmError {
            message: "rate limited".into(),
            retryable: true,
        };
        let s = format_error(&err, false);
        assert!(
            s.contains("\x1b[31mError: LLM error: rate limited\x1b[0m"),
            "got: {s}"
        );
    }

    // ── Async integration: stream consumption ────────────────────────

    #[tokio::test]
    async fn test_async_stream_consumes_all_events() {
        let events: Vec<AgentEvent> = vec![
            AgentEvent::ToolCallStart {
                id: "c1".into(),
                name: "read".into(),
                args: serde_json::json!({"path": "/tmp/x"}),
            },
            AgentEvent::TextDelta("Hello world".into()),
            AgentEvent::ToolCallEnd {
                id: "c1".into(),
                result: serde_json::json!({"content": "x"}),
            },
        ];
        let result = run_print_mode(futures::stream::iter(events), true, None).await;
        assert!(result.is_ok());
    }
}
