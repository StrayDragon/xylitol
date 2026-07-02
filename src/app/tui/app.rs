//! TUI application state machine.
//!
//! Holds the mutable state the render loop reads: input buffer, the current
//! streaming assistant text, spinner/tool status, and whether a turn is active.
//! The render loop (in `mod.rs`) drains the agent's [`XyEvent`] stream via a
//! spawn task that forwards into the shared `Msg` channel.

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::core::driver::EventStream;
use crate::app::tui::render::{RenderedLine, StatusLine, xyevent_to_rendered};
use crate::domain::lifecycle::XyEvent;

pub(crate) const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// ── StreamBuffer: newline-gated streaming buffer (c365, codex 极简版) ──────
//
// 流式 token 累积进 buffer；只在换行边界内的完整行可 drain 为稳定行（立即 commit 进
// scrollback）；未换行的尾部是 mutable last line（每帧在 tail 区顶部重绘，视觉上紧贴
// scrollback 生长）。对标 codex MarkdownStreamCollector（markdown_stream.rs:87-96）的纯
// 文本极简版——无 markdown 解析、无 table holdback、无动画 chunking。

/// Newline-gated streaming buffer. Grows with each token push; drains complete
/// (newline-terminated) lines as stable lines; the un-terminated remainder is the
/// mutable last line redrawn each frame.
#[derive(Default)]
pub struct StreamBuffer {
    buffer: String,
    /// Byte offset of the last drained newline boundary.
    committed_len: usize,
}

impl StreamBuffer {
    /// Append a streaming delta (may contain partial text, newlines, or multiple lines).
    pub fn push(&mut self, delta: &str) {
        self.buffer.push_str(delta);
    }

    /// Drain all complete (newline-terminated) lines since the last call, advancing
    /// the committed boundary. Returns the line texts (without the trailing newline).
    pub fn drain_complete_lines(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        while let Some(rel) = self.buffer[self.committed_len..].find('\n') {
            let abs = self.committed_len + rel;
            let line: String = self.buffer[self.committed_len..abs].to_string();
            self.committed_len = abs + 1;
            out.push(line);
        }
        out
    }

    /// The un-terminated tail (mutable last line content). Redrawn each frame in
    /// the tail region's top row, visually continuous with the scrollback above.
    pub fn pending_tail(&self) -> &str {
        &self.buffer[self.committed_len..]
    }

    /// On turn end: return any residual un-terminated text as a final line, then
    /// reset the buffer for the next turn. Returns None if nothing residual.
    pub fn finalize(&mut self) -> Option<String> {
        let tail = self.pending_tail().to_string();
        self.buffer.clear();
        self.committed_len = 0;
        if tail.is_empty() { None } else { Some(tail) }
    }
}

/// The live TUI state.
#[derive(Default)]
pub struct TuiApp {
    input: String,
    /// Newline-gated streaming buffer (c365): complete lines drain to scrollback
    /// immediately; the un-terminated tail is the mutable last line.
    stream_buf: StreamBuffer,
    /// Accumulated finalized assistant text for the current turn (kept for
    /// compatibility; the live streaming view comes from stream_buf.pending_tail).
    finalized: String,
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
        self.stream_buf = StreamBuffer::default();
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

    /// Apply an XyEvent; return finalized [`RenderedLine`]s to commit to
    /// scrollback (if any). The un-terminated streaming tail is NOT returned
    /// here — it lives in `pending_tail()` and is rendered by the `MutableLine`
    /// widget each frame (c365 buffer route).
    ///
    /// Business state updates (pending accumulation, status line) stay here; the
    /// XyEvent→UI-data translation goes through the `xyevent_to_rendered` seam
    /// (spec tui42). Spinner animation is driven by the steady Tick message
    /// (see `mod.rs`), NOT here.
    pub fn handle_xy_event(&mut self, event: XyEvent) -> Vec<RenderedLine> {
        // Translate via the seam first: the only place XyEvent is matched for
        // rendering purposes. Business state updates below consume the event by
        // reference without re-deriving render lines.
        let mut rendered = xyevent_to_rendered(&event);
        match &event {
            XyEvent::TextDelta(text) => {
                // Newline-gated incremental commit (c365): push the delta, drain
                // complete lines immediately (they commit to scrollback now, not
                // at turn end). The un-terminated tail stays in stream_buf and is
                // redrawn each frame as the mutable last line.
                self.stream_buf.push(text);
                let complete = self.stream_buf.drain_complete_lines();
                rendered.extend(complete.into_iter().map(RenderedLine::AssistantText));
            }
            XyEvent::ToolExecutionStart { name, .. } => {
                self.status = Some(StatusLine::new("⚙", format!(" running {name}")));
            }
            XyEvent::ToolExecutionUpdate { .. } => {}
            XyEvent::ToolExecutionEnd { name, .. } => {
                self.status = Some(StatusLine::new("✓", format!(" {name} done")));
            }
            XyEvent::TurnEnd { .. } => {
                // Flush any residual un-terminated text as a final line.
                if let Some(tail) = self.stream_buf.finalize() {
                    rendered.push(RenderedLine::AssistantText(tail));
                }
            }
            XyEvent::ThinkingDelta(_)
            | XyEvent::ModelSelect { .. }
            | XyEvent::Error(_)
            | XyEvent::TurnStart { .. } => {
                // No additional business state; render lines already produced by
                // the seam (ModelSelect/Error) or intentionally none (the rest).
            }
            // Degrade gracefully on unhandled variants: no panic.
            _ => {}
        }
        rendered
    }

    /// Called when the turn stream ends; resets streaming state.
    ///
    /// Also clears the stream buffer defensively (修复 c340 §7 #4): if the stream
    /// ended without a TurnEnd (e.g. abort mid-stream), leftover tail text would
    /// otherwise linger until the next submit. The normal TurnEnd path already
    /// drains via `handle_xy_event` → `finalize`, so this is a no-op in the
    /// common case.
    pub fn end_stream(&mut self) {
        self.streaming = false;
        self.status = None;
        self.stream_buf = StreamBuffer::default();
    }

    pub fn turn_done(&self, event: &XyEvent) -> bool {
        matches!(event, XyEvent::TurnEnd { .. })
    }

    // ── tail presentation ───────────────────────────────────────
    /// The un-terminated streaming tail (mutable last line). Redrawn each frame
    /// at the top of the tail region, visually continuous with the scrollback.
    /// Returns None when the tail is empty.
    pub fn pending_tail(&self) -> Option<&str> {
        let tail = self.stream_buf.pending_tail();
        if tail.is_empty() { None } else { Some(tail) }
    }

    /// The current spinner animation frame index (advanced by `tick_spinner`).
    /// Read by the `ThinkingIndicator` widget to pick the glyph.
    pub fn spinner_idx(&self) -> usize {
        self.spinner_idx
    }

    pub fn status_line(&self) -> Option<&StatusLine> {
        self.status.as_ref()
    }

    // ── internals ───────────────────────────────────────────────
    pub(crate) fn tick_spinner(&mut self) {
        self.spinner_idx = (self.spinner_idx + 1) % SPINNER.len();
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
        assert!(app.pending_tail().unwrap().contains("world"));
    }

    #[test]
    fn turn_end_flushes_remaining_pending() {
        let mut app = TuiApp::default();
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        let end = app.handle_xy_event(XyEvent::TurnEnd { turn_index: 0 });
        assert_eq!(end.len(), 1);
        // 修复 c340 §7 #4: after TurnEnd the pending buffer must be empty so
        // nothing lingers in the tail.
        assert!(app.pending_tail().is_none());
    }

    #[test]
    fn end_stream_clears_pending() {
        // 修复 c340 §7 #4: end_stream (called on abort without TurnEnd) must
        // also clear pending so no reply text lingers in the tail.
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        assert!(app.pending_tail().is_some());
        app.end_stream();
        assert!(app.pending_tail().is_none());
        assert!(!app.is_streaming());
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

    // ── StreamBuffer newline-gated behavior (c365) ──────────────────

    #[test]
    fn stream_push_drains_complete_lines_on_newline() {
        let mut buf = StreamBuffer::default();
        buf.push("hello\nworld");
        let complete = buf.drain_complete_lines();
        assert_eq!(complete, vec!["hello".to_string()]);
        // The un-terminated remainder is the mutable last line.
        assert_eq!(buf.pending_tail(), "world");
    }

    #[test]
    fn stream_pending_tail_grows_without_commit() {
        let mut buf = StreamBuffer::default();
        buf.push("abc");
        assert!(
            buf.drain_complete_lines().is_empty(),
            "no newline → no drain"
        );
        assert_eq!(buf.pending_tail(), "abc");
        buf.push("def");
        assert_eq!(buf.pending_tail(), "abcdef", "tail grows with each push");
    }

    #[test]
    fn stream_finalize_flushes_residual() {
        let mut buf = StreamBuffer::default();
        buf.push("a\nb");
        let _ = buf.drain_complete_lines(); // drains "a"
        assert_eq!(buf.finalize(), Some("b".to_string()));
        // After finalize the buffer is reset for the next turn.
        assert_eq!(buf.pending_tail(), "");
    }

    #[test]
    fn stream_finalize_none_when_empty() {
        let mut buf = StreamBuffer::default();
        buf.push("a\n");
        let _ = buf.drain_complete_lines();
        assert_eq!(buf.finalize(), None);
    }
}
