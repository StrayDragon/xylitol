//! TUI application state machine.
//!
//! Holds the mutable state the render loop reads: input buffer, the current
//! streaming assistant text, spinner/tool status, and whether a turn is active.
//! The render loop (in `mod.rs`) drains the agent's [`XyEvent`] stream via a
//! spawn task that forwards into the shared `Msg` channel.

use std::sync::Arc;

use ratatui_core::style::Style;
use ratatui_core::text::Line;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::core::driver::EventStream;
use crate::app::tui::render::{StatusLine, commit_lines_for};
use crate::domain::lifecycle::XyEvent;

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// The live TUI state.
#[derive(Default)]
pub struct TuiApp {
    input: String,
    /// Accumulated finalized assistant text for the current turn (committed to
    /// scrollback on newline boundaries / turn end).
    finalized: String,
    /// Current in-progress line (not yet newline-terminated).
    pending: String,
    streaming: bool,
    spinner_idx: usize,
    status: Option<StatusLine>,
}

impl TuiApp {
    // ── input ───────────────────────────────────────────────────
    pub fn input_buffer(&self) -> &str {
        &self.input
    }
    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
    }
    pub fn backspace(&mut self) {
        self.input.pop();
    }
    pub fn take_input(&mut self) -> String {
        std::mem::take(&mut self.input)
    }

    // ── streaming lifecycle ─────────────────────────────────────
    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    /// Mark a turn active. The caller spawns the drain task that forwards
    /// `XyEvent`s into the shared `Msg` channel.
    pub fn start_stream(&mut self) {
        self.streaming = true;
        self.finalized.clear();
        self.pending.clear();
        self.status = Some(StatusLine::new(" ", "thinking…"));
    }

    /// Spawn a task draining `stream` into `tx`, one `XyEvent` per message,
    /// until `TurnEnd` or cancellation. This decouples the async `EventStream`
    /// from the synchronous render loop.
    pub fn spawn_drain(
        stream: EventStream,
        tx: mpsc::UnboundedSender<XyEvent>,
        cancel: Arc<CancellationToken>,
    ) {
        tokio::spawn(async move {
            use futures::StreamExt;
            tokio::pin!(stream);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    item = stream.next() => {
                        match item {
                            Some(ev) => {
                                let is_end = matches!(ev, XyEvent::TurnEnd { .. });
                                if tx.send(ev).is_err() {
                                    break;
                                }
                                if is_end {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
        });
    }

    /// Apply an XyEvent; return lines to commit to scrollback (if any).
    /// Apply an XyEvent; return lines to commit to scrollback (if any).
    /// Spinner animation is driven by the steady Tick message (see
    /// `mod.rs`), NOT here, so it keeps a constant speed independent of
    /// event arrival rate.
    pub fn handle_xy_event(&mut self, event: XyEvent) -> Vec<Line<'static>> {
        match &event {
            XyEvent::TextDelta(text) => {
                self.pending.push_str(text);
                // Commit on newline boundaries: flush complete lines.
                self.flush_complete_pending()
            }
            XyEvent::ThinkingDelta(_) => {
                // Thinking is not committed to scrollback in MVP (lives in tail
                // status only). Keep degrade-safe: ignore content.
                Vec::new()
            }
            XyEvent::ToolExecutionStart { name, .. } => {
                self.status = Some(StatusLine::new("⚙", format!(" running {name}")));
                Vec::new()
            }
            XyEvent::ToolExecutionUpdate { .. } => Vec::new(),
            XyEvent::ToolExecutionEnd { name, .. } => {
                self.status = Some(StatusLine::new("✓", format!(" {name} done")));
                commit_lines_for(&event)
            }
            XyEvent::ModelSelect { .. } | XyEvent::Error(_) => commit_lines_for(&event),
            XyEvent::TurnStart { .. } => Vec::new(),
            XyEvent::TurnEnd { .. } => {
                // Commit any remaining pending text as a final line.
                let mut out = Vec::new();
                if !self.pending.is_empty() {
                    let line = self.pending.clone();
                    self.pending.clear();
                    out.push(Line::styled(line, Style::default()));
                }
                out
            }
            // Degrade gracefully on unhandled variants: no panic.
            _ => Vec::new(),
        }
    }

    /// Called when the turn stream ends; resets streaming state.
    pub fn end_stream(&mut self) {
        self.streaming = false;
        self.status = None;
    }

    pub fn turn_done(&self, event: &XyEvent) -> bool {
        matches!(event, XyEvent::TurnEnd { .. })
    }

    // ── tail presentation ───────────────────────────────────────
    pub fn current_streaming_line(&self) -> Option<&str> {
        if self.pending.is_empty() {
            None
        } else {
            Some(&self.pending)
        }
    }

    /// The thinking/loading indicator label shown while streaming: an animated
    /// spinner glyph + a label. Prefers a concrete tool/status line when one
    /// is active (e.g. "running read_file"), else shows "Thinking…".
    pub fn indicator_label(&self) -> String {
        let spinner = SPINNER[self.spinner_idx];
        let label = self
            .status
            .as_ref()
            .map(|s| s.label().to_string())
            .unwrap_or_else(|| "Thinking…".to_string());
        format!("{spinner} {label}")
    }

    pub fn status_line(&self) -> Option<&StatusLine> {
        self.status.as_ref()
    }

    // ── internals ───────────────────────────────────────────────
    pub(crate) fn tick_spinner(&mut self) {
        self.spinner_idx = (self.spinner_idx + 1) % SPINNER.len();
    }

    /// Move any newline-terminated lines from `pending` into the returned vec.
    fn flush_complete_pending(&mut self) -> Vec<Line<'static>> {
        let mut out = Vec::new();
        while let Some(idx) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=idx).collect();
            let trimmed = line.trim_end_matches('\n');
            out.push(Line::styled(trimmed.to_string(), Style::default()));
        }
        out
    }

    /// Expose the current spinner glyph (for tests / status rendering).
    #[cfg(test)]
    pub fn spinner_glyph(&self) -> &'static str {
        SPINNER[self.spinner_idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textdelta_commits_on_newline_and_keeps_partial() {
        let mut app = TuiApp::default();
        let a = app.handle_xy_event(XyEvent::TextDelta("hello\nworld".into()));
        assert_eq!(a.len(), 1, "one complete line committed");
        assert!(app.current_streaming_line().unwrap().contains("world"));
    }

    #[test]
    fn turn_end_flushes_remaining_pending() {
        let mut app = TuiApp::default();
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        let end = app.handle_xy_event(XyEvent::TurnEnd { turn_index: 0 });
        assert_eq!(end.len(), 1);
    }

    #[test]
    fn tool_end_produces_summary_line() {
        let mut app = TuiApp::default();
        let lines = app.handle_xy_event(XyEvent::ToolExecutionEnd {
            id: "t1".into(),
            name: "read_file".into(),
            result: "ok".into(),
            is_error: false,
        });
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn unhandled_event_does_not_panic() {
        let mut app = TuiApp::default();
        let lines = app.handle_xy_event(XyEvent::CompactionStart {
            reason: "test".into(),
        });
        assert!(lines.is_empty());
    }
}
