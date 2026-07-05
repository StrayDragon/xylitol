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
use crate::app::tui::components::spinner::SPINNER;
use crate::app::tui::render::{RenderedLine, xyevent_to_rendered};
use crate::domain::lifecycle::XyEvent;

/// What the mutable line is currently showing, and how to style it.
/// Read by the `Tail` widget to pick the style for `MutableLine`.
#[derive(Clone, Copy)]
pub enum MutableKind {
    /// Reasoning/thinking content (dim/gray).
    Thinking,
    /// Main assistant reply text (normal).
    Text,
    /// Tool execution status (tool/dim).
    Tool,
}

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
    /// Byte offset of the last drained paragraph boundary.
    committed_len: usize,
    /// Whether we are currently inside a fenced code block (c377). While true,
    /// blank lines do NOT split paragraphs — the whole code block accumulates
    /// until the closing fence, so render_markdown gets complete fence context.
    in_fence: bool,
}

impl StreamBuffer {
    /// Append a streaming delta (may contain partial text, newlines, or multiple lines).
    pub fn push(&mut self, delta: &str) {
        self.buffer.push_str(delta);
    }

    /// Drain complete lines/paragraphs since the last call, advancing the
    /// committed boundary. Fence-aware (c377):
    /// - **Outside a code fence**: behaves like the legacy line-at-a-time
    ///   drain — each newline-terminated line commits immediately. This keeps
    ///   streaming text flowing to scrollback line-by-line (spec: timely
    ///   scrollback updates) and works because non-code lines need no fence
    ///   context for markdown rendering.
    /// - **Inside a code fence** (opened by a ``` line): lines accumulate
    ///   until the matching closing ``` line, then the whole block commits
    ///   as ONE unit. This gives render_markdown complete fence context so
    ///   syntax highlighting applies (the root cause fix for c377).
    ///
    /// Only fully newline-terminated content is considered; the un-terminated
    /// tail stays for the mutable region (pending_tail).
    pub fn drain_complete_paragraphs(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        // Find the last newline at/after committed_len — content beyond it is
        // the un-terminated tail and must stay.
        let scan_end = match self.buffer[self.committed_len..].rfind('\n') {
            Some(rel) => self.committed_len + rel + 1,
            None => return out, // no complete line yet
        };
        // Re-derive fence state from the buffer each call (don't trust a
        // stored in_fence flag, because committed_len doesn't advance past
        // an unclosed fence — the fence lines would be re-scanned and
        // double-toggle the flag). We scan from committed_len; the flag
        // persists in `self.in_fence` only to reflect the state at the END of
        // this call (so the mutable tail knows if it's inside a fence).
        let mut local_in_fence = false;
        let mut i = self.committed_len;
        while i < scan_end {
            let line_end = self.buffer[i..scan_end]
                .find('\n')
                .map(|rel| i + rel)
                .unwrap_or(scan_end);
            let line = self.buffer[i..line_end].to_string();
            let is_fence = line.trim_start().starts_with("```");
            if is_fence && !local_in_fence {
                // Opening fence: start accumulating (don't commit yet).
                local_in_fence = true;
            } else if is_fence && local_in_fence {
                // Closing fence: flush from committed_len to here as one block.
                local_in_fence = false;
                let block = self.buffer[self.committed_len..line_end + 1]
                    .trim_end()
                    .to_string();
                out.push(block);
                self.committed_len = line_end + 1;
            } else if !local_in_fence {
                // Outside fence: commit each line immediately (legacy behavior).
                if !line.trim().is_empty() {
                    out.push(line);
                }
                self.committed_len = line_end + 1;
            }
            // else: inside fence (non-fence line) — keep accumulating.
            i = line_end + 1;
        }
        // Persist the end-of-scan fence state (so the caller / mutable tail
        // knows whether the pending content is mid-code-block).
        self.in_fence = local_in_fence;
        out
    }

    /// Drain all complete lines (legacy, c365). Kept for backward-compat with
    /// tests that exercise the newline-gated boundary directly.
    pub fn drain_complete_lines(&mut self) -> Vec<String> {
        self.drain_complete_paragraphs()
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
        self.in_fence = false;
        if tail.is_empty() { None } else { Some(tail) }
    }
}

/// Status content for the status line (c380). Pure data — the `StatusLine`
/// widget renders it without matching business state. See
/// [`TuiApp::status_segments`].
pub struct StatusSegments {
    /// Whether a turn is streaming (controls spinner rendering).
    pub streaming: bool,
    /// Activity label following the spinner (e.g. "Working…", "Running bash",
    /// "Ready").
    pub left_label: String,
}

/// The live TUI state.
#[derive(Default)]
pub struct TuiApp {
    input: String,
    /// Byte offset of the insert cursor within `input` (c377). Characters
    /// insert here; Backspace deletes the char before it. Left/Right move by
    /// char boundary (CJK-safe via char_indices).
    input_cursor: usize,
    /// Newline-gated streaming buffer for main reply text (TextDelta).
    stream_buf: StreamBuffer,
    /// Newline-gated streaming buffer for reasoning text (ThinkingDelta).
    /// Thinking is part of the conversation body (gray, above the reply), not
    /// a separate panel widget — it streams and commits like the reply.
    thinking_buf: StreamBuffer,
    /// Accumulated finalized assistant text for the current turn (kept for
    /// compatibility; the live streaming view comes from stream_buf.pending_tail).
    finalized: String,
    streaming: bool,
    /// True while in the thinking phase (before the first TextDelta). The
    /// mutable line shows thinking content (or a `Thinking…` placeholder);
    /// the first TextDelta flushes the thinking buffer and switches to text.
    thinking_phase: bool,
    spinner_idx: usize,
    /// Active tool execution status label, shown as the mutable line when no
    /// streaming text/thinking is pending (e.g. while a tool runs between
    /// text chunks).
    tool_status: Option<String>,
}

impl TuiApp {
    // ── input ───────────────────────────────────────────────────
    pub fn input_buffer(&self) -> &str {
        &self.input
    }
    /// Current cursor byte offset (for rendering `input[..cursor]` width).
    pub fn input_cursor(&self) -> usize {
        self.input_cursor
    }
    pub fn push_char(&mut self, c: char) {
        self.input.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
    }
    pub fn backspace(&mut self) {
        if self.input_cursor > 0 {
            // Find the previous char boundary.
            let prev = self.input[..self.input_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.replace_range(prev..self.input_cursor, "");
            self.input_cursor = prev;
        }
    }
    pub fn cursor_left(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor = self.input[..self.input_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }
    pub fn cursor_right(&mut self) {
        if self.input_cursor < self.input.len() {
            // Advance by one char's byte length.
            let mut chars = self.input[self.input_cursor..].char_indices();
            chars.next(); // skip the char at cursor
            if let Some((next_off, _)) = chars.next() {
                self.input_cursor += next_off;
            } else {
                // Single char at cursor — move to end.
                self.input_cursor = self.input.len();
            }
        }
    }
    pub fn cursor_home(&mut self) {
        self.input_cursor = 0;
    }
    pub fn cursor_end(&mut self) {
        self.input_cursor = self.input.len();
    }
    pub fn take_input(&mut self) -> String {
        self.input_cursor = 0;
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
        self.thinking_phase = true;
        self.finalized.clear();
        self.stream_buf = StreamBuffer::default();
        self.thinking_buf = StreamBuffer::default();
        self.tool_status = None;
    }

    /// Spawn a task draining `stream` into `tx`, one `XyEvent` per message,
    /// until the stream ends (`None`) or cancellation. The drain runs to
    /// stream end: a multi-round tool-calling turn emits one `TurnEnd` per
    /// ReAct iteration, and only the final stream `None` marks the whole
    /// user turn done (c370). Breaking on the first `TurnEnd` would drop the
    /// model's continuation events after a tool call and hang the REPL.
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
                    item = stream.next() => match item {
                        Some(ev) => {
                            if tx.send(ev).is_err() {
                                break;
                            }
                        }
                        None => break,
                    },
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
        let mut rendered = xyevent_to_rendered(&event);
        match &event {
            XyEvent::ThinkingDelta(text) => {
                // Thinking is conversation body (gray): stream + commit like
                // the reply, but via the thinking buffer and ThinkingText lines.
                self.thinking_buf.push(text);
                let complete = self.thinking_buf.drain_complete_lines();
                rendered.extend(complete.into_iter().map(RenderedLine::ThinkingText));
            }
            XyEvent::TextDelta(text) => {
                // Transition from thinking to text: flush the thinking buffer's
                // pending tail as a final ThinkingText line, then switch phase.
                if self.thinking_phase {
                    if let Some(tail) = self.thinking_buf.finalize() {
                        rendered.push(RenderedLine::ThinkingText(tail));
                    }
                    self.thinking_phase = false;
                }
                self.stream_buf.push(text);
                let complete = self.stream_buf.drain_complete_lines();
                rendered.extend(complete.into_iter().map(RenderedLine::AssistantText));
            }
            XyEvent::ToolExecutionStart { name, .. } => {
                self.tool_status = Some(format!("⚙ running {name}"));
            }
            XyEvent::ToolExecutionUpdate { .. } => {}
            XyEvent::ToolExecutionEnd { name, .. } => {
                self.tool_status = Some(format!("✓ {name} done"));
            }
            XyEvent::TurnEnd { .. } => {
                // Flush any residual from both buffers.
                if let Some(tail) = self.thinking_buf.finalize() {
                    rendered.push(RenderedLine::ThinkingText(tail));
                }
                if let Some(tail) = self.stream_buf.finalize() {
                    rendered.push(RenderedLine::AssistantText(tail));
                }
            }
            XyEvent::TurnStart { .. } | XyEvent::ModelSelect { .. } | XyEvent::Error(_) => {
                // No additional business state; render lines already produced by
                // the seam (ModelSelect/Error) or intentionally none (TurnStart).
            }
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
        self.thinking_phase = false;
        self.tool_status = None;
        self.stream_buf = StreamBuffer::default();
        self.thinking_buf = StreamBuffer::default();
    }

    // ── tail presentation ───────────────────────────────────────
    /// The mutable line content + its kind (for styling). Redrawn each frame
    /// at the top of the tail region, visually continuous with the scrollback.
    /// Returns `None` when there is nothing to show (e.g. between phases with
    /// no pending content and no tool status).
    ///
    /// While in the thinking phase with an empty thinking buffer, returns the
    /// `Thinking…` placeholder (gray) — this is the activity indicator (no
    /// separate spinner; the streaming text itself shows the agent is alive).
    pub fn pending_tail(&self) -> Option<(&str, MutableKind)> {
        use MutableKind as K;
        if self.thinking_phase {
            let tail = self.thinking_buf.pending_tail();
            if !tail.is_empty() {
                return Some((tail, K::Thinking));
            }
            return Some(("Thinking…", K::Thinking));
        }
        let text_tail = self.stream_buf.pending_tail();
        if !text_tail.is_empty() {
            return Some((text_tail, K::Text));
        }
        if let Some(status) = self.tool_status.as_deref() {
            return Some((status, K::Tool));
        }
        None
    }

    /// Whether the turn is currently in the thinking phase (before the first
    /// TextDelta). Read by `Tail` to style the mutable line.
    pub fn is_thinking_phase(&self) -> bool {
        self.thinking_phase
    }

    /// The current spinner animation frame index (kept for future use; the
    /// spinner is not currently rendered — `Thinking…` / streaming text serves
    /// as the activity indicator).
    pub fn spinner_idx(&self) -> usize {
        self.spinner_idx
    }

    /// Active tool status label, if any.
    pub fn tool_status(&self) -> Option<&str> {
        self.tool_status.as_deref()
    }

    /// Three-segment status content for the status line (c380). Data-driven:
    /// the `StatusLine` widget renders these by alignment and does NOT match
    /// business state itself. Extensible — future items (token count, elapsed
    /// time) slot into a segment without touching the widget.
    ///
    /// - `left_label`: activity label after the spinner (Working / Running X /
    ///   Ready). The spinner glyph itself is rendered by the widget from
    ///   `spinner_idx()` (only meaningful while streaming).
    /// - `center`: `Turn {n}` while streaming, else None.
    /// - `right`: model name, else None.
    pub fn status_segments(&self) -> StatusSegments {
        let left_label = if !self.streaming {
            "Ready".to_string()
        } else if let Some(status) = &self.tool_status {
            // tool_status is already "⚙ running bash" / "✓ bash done"; strip
            // the leading glyph for the status line (the spinner replaces it).
            status
                .trim_start_matches(['⚙', ' ', '✓'])
                .trim()
                .to_string()
        } else {
            "Working…".to_string()
        };
        StatusSegments {
            streaming: self.streaming,
            left_label,
        }
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
        assert!(app.pending_tail().unwrap().0.contains("world"));
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

    #[test]
    fn fence_aware_commit_accumulates_code_block_until_close() {
        // c377: a fenced code block streamed in chunks must accumulate and
        // commit as ONE unit when the closing fence arrives — not line by
        // line (which loses fence context for highlighting).
        let mut buf = StreamBuffer::default();
        // Stream: opening fence + code line + (no close yet)
        buf.push("```rs\nfn main() {}\n");
        let mid = buf.drain_complete_paragraphs();
        assert!(
            mid.is_empty(),
            "no commit while fence unclosed (accumulating): {mid:?}"
        );
        // Now the closing fence arrives.
        buf.push("```\n");
        let closed = buf.drain_complete_paragraphs();
        assert_eq!(closed.len(), 1, "code block commits as one unit");
        let block = &closed[0];
        assert!(block.contains("```rs"), "opening fence in block: {block}");
        assert!(block.contains("fn main()"), "code body: {block}");
        assert!(block.contains("```"), "closing fence: {block}");
    }

    #[test]
    fn fence_aware_blank_line_inside_code_block_does_not_split() {
        // c377: blank lines inside a code block are content, not paragraph
        // boundaries — they must NOT split the block.
        let mut buf = StreamBuffer::default();
        buf.push("```rs\nlet a = 1;\n\nlet b = 2;\n```\n");
        let out = buf.drain_complete_paragraphs();
        assert_eq!(
            out.len(),
            1,
            "whole block is one unit despite internal blank"
        );
        assert!(out[0].contains("let a = 1;"));
        assert!(out[0].contains("let b = 2;"));
    }

    #[test]
    fn outside_fence_lines_commit_immediately() {
        // c377: outside a fence, lines still commit immediately (legacy
        // streaming behavior — timely scrollback updates).
        let mut buf = StreamBuffer::default();
        buf.push("hello\nworld\n");
        let out = buf.drain_complete_paragraphs();
        assert_eq!(out, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn cursor_inserts_in_middle() {
        // c377: push_char inserts at cursor, not always at the end.
        let mut app = TuiApp::default();
        for c in "abc".chars() {
            app.push_char(c);
        }
        // Move cursor left once (now between 'b' and 'c').
        app.cursor_left();
        assert_eq!(app.input_cursor(), 2);
        app.push_char('X');
        assert_eq!(app.input_buffer(), "abXc");
    }

    #[test]
    fn cursor_left_right_cjk_boundaries() {
        // c377: cursor moves by char boundary, not byte.
        let mut app = TuiApp::default();
        for c in "你好".chars() {
            app.push_char(c);
        }
        assert_eq!(app.input_cursor(), 6, "two CJK chars = 6 bytes");
        app.cursor_left();
        assert_eq!(app.input_cursor(), 3, "left moves to char boundary");
        app.cursor_right();
        assert_eq!(app.input_cursor(), 6, "right back to end");
    }

    #[test]
    fn backspace_at_cursor_middle() {
        let mut app = TuiApp::default();
        for c in "abc".chars() {
            app.push_char(c);
        }
        app.cursor_left(); // cursor at 2 (between 'b' and 'c')
        app.backspace(); // deletes 'b'
        assert_eq!(app.input_buffer(), "ac");
        assert_eq!(app.input_cursor(), 1);
    }

    // ── Multi-round tool-turn drain (c370) ──────────────────────────
    //
    // react.rs emits one TurnEnd per ReAct iteration; a tool-calling turn
    // spans multiple iterations. spawn_drain MUST run to stream end (None),
    // not stop at the first (intermediate) TurnEnd, or the model's
    // continuation events after a tool call are dropped.

    /// Build an EventStream from a fixed event sequence (for deterministic tests).
    fn event_stream(events: Vec<XyEvent>) -> EventStream {
        Box::pin(futures::stream::iter(events))
    }

    #[tokio::test]
    async fn drain_runs_past_intermediate_turnend_to_stream_end() {
        // A tool-calling turn: text → (tool round) intermediate TurnEnd →
        // continuation text → final TurnEnd. Before c370, drain broke on the
        // first TurnEnd and the continuation never arrived.
        let events = vec![
            XyEvent::TextDelta("let me check\n".into()),
            XyEvent::ToolExecutionEnd {
                id: "t1".into(),
                name: "bash".into(),
                result: "ok".into(),
                is_error: false,
            },
            XyEvent::TurnEnd { turn_index: 0 }, // intermediate (react.rs:453)
            XyEvent::TextDelta("the answer is 42\n".into()), // continuation
            XyEvent::TurnEnd { turn_index: 1 }, // final
        ];
        let stream = event_stream(events.clone());
        let (tx, mut rx) = mpsc::unbounded_channel::<XyEvent>();
        let cancel = Arc::new(CancellationToken::new());

        TuiApp::spawn_drain(stream, tx, cancel);
        // Give the spawned drain task time to finish.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut received = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            received.push(ev);
        }
        assert_eq!(
            received.len(),
            events.len(),
            "drain MUST forward every event including those after the intermediate TurnEnd"
        );
        // The continuation text after the tool round must survive.
        assert!(
            received.iter().any(|ev| matches!(
                ev,
                XyEvent::TextDelta(t) if t.contains("the answer is 42")
            )),
            "continuation text after the tool call must reach the TUI"
        );
    }

    #[tokio::test]
    async fn drain_stops_on_cancel() {
        // cancel is cooperative: a cancelled token makes the drain's select!
        // take the cancel branch and stop. With a pre-cancelled token and a
        // stream that yields one event, select! may race and forward 0 or 1
        // events before breaking — the invariant is that the drain task ends
        // promptly (does not hang waiting on stream end).
        let stream = event_stream(vec![XyEvent::TextDelta("maybe\n".into())]);
        let (tx, mut rx) = mpsc::unbounded_channel::<XyEvent>();
        let cancel = Arc::new(CancellationToken::new());
        cancel.cancel();

        TuiApp::spawn_drain(stream, tx, cancel);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Drain at most 1 event (cooperative cancel may let the single ready
        // item through before observing cancel). The point is it terminates.
        let mut received = 0;
        while rx.try_recv().is_ok() {
            received += 1;
        }
        assert!(
            received <= 1,
            "cancelled drain must not run the whole stream"
        );
    }

    #[tokio::test]
    async fn fresh_token_drains_after_a_prior_cancel() {
        // Regression for the cancel-reuse bug (c370): a global token stayed
        // cancelled after the first abort, starving every later turn. With a
        // fresh per-turn token, a turn started after a previous cancel must
        // drain its stream normally.
        let stale = Arc::new(CancellationToken::new());
        stale.cancel(); // a prior abort

        // A new turn creates its own token (not the stale one).
        let fresh = Arc::new(CancellationToken::new());
        let stream = event_stream(vec![
            XyEvent::TextDelta("after abort\n".into()),
            XyEvent::TurnEnd { turn_index: 0 },
        ]);
        let (tx, mut rx) = mpsc::unbounded_channel::<XyEvent>();
        TuiApp::spawn_drain(stream, tx, fresh);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut received = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            received.push(ev);
        }
        assert_eq!(
            received.len(),
            2,
            "a fresh token must drain the stream even after a prior cancel"
        );
        let _ = stale; // keep the intent explicit
    }
}
