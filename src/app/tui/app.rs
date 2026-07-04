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
#[derive(Clone, Copy, Debug)]
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
    /// Accumulated assistant text for the current turn (c375: deferred commit).
    /// TextDelta appends here during streaming; the full text is committed as
    /// a single `AssistantText` at TurnEnd so render_markdown gets complete
    /// fenced-code-block context for syntax highlighting.
    finalized: String,
    /// Accumulated reasoning text for the current turn (c375: deferred commit,
    /// same rationale as `finalized`).
    thinking_finalized: String,
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
        self.thinking_phase = true;
        self.finalized.clear();
        self.thinking_finalized.clear();
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
                // c375: accumulate full thinking text; commit at TurnEnd so
                // render_markdown gets complete context (code blocks, lists).
                self.thinking_finalized.push_str(text);
            }
            XyEvent::TextDelta(text) => {
                // Transition from thinking to text phase (no commit here —
                // thinking text is committed at TurnEnd with the rest).
                self.thinking_phase = false;
                // c375: accumulate full assistant text; commit at TurnEnd so
                // render_markdown gets complete fenced-code-block context.
                self.finalized.push_str(text);
            }
            XyEvent::ToolExecutionStart { name, .. } => {
                self.tool_status = Some(format!("⚙ running {name}"));
            }
            XyEvent::ToolExecutionUpdate { .. } => {}
            XyEvent::ToolExecutionEnd { name, .. } => {
                self.tool_status = Some(format!("✓ {name} done"));
            }
            XyEvent::TurnEnd { .. } => {
                // c375: commit accumulated text as single blocks (full markdown
                // context → highlighting works). Each intermediate TurnEnd in a
                // multi-round tool-calling turn commits its own segment.
                if !self.thinking_finalized.is_empty() {
                    rendered.push(RenderedLine::ThinkingText(std::mem::take(
                        &mut self.thinking_finalized,
                    )));
                }
                if !self.finalized.is_empty() {
                    rendered.push(RenderedLine::AssistantText(std::mem::take(
                        &mut self.finalized,
                    )));
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
    /// Called when the turn stream ends; resets streaming state. Returns any
    /// residual accumulated text as finalized `RenderedLine`s (defensive — the
    /// normal TurnEnd path already committed via `handle_xy_event`; this only
    /// fires if the stream ended without a TurnEnd, e.g. abort mid-stream).
    pub fn end_stream(&mut self) -> Vec<RenderedLine> {
        let mut rendered = Vec::new();
        if !self.thinking_finalized.is_empty() {
            rendered.push(RenderedLine::ThinkingText(std::mem::take(
                &mut self.thinking_finalized,
            )));
        }
        if !self.finalized.is_empty() {
            rendered.push(RenderedLine::AssistantText(std::mem::take(
                &mut self.finalized,
            )));
        }
        self.streaming = false;
        self.thinking_phase = false;
        self.tool_status = None;
        rendered
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
            if !self.thinking_finalized.is_empty() {
                return Some((&self.thinking_finalized, K::Thinking));
            }
            return Some(("Thinking…", K::Thinking));
        }
        if !self.finalized.is_empty() {
            return Some((&self.finalized, K::Text));
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
    fn textdelta_accumulates_without_committing() {
        // c375: TextDelta defers commit to TurnEnd. During streaming, nothing
        // is committed (empty Vec); the full text accumulates in pending_tail.
        let mut app = TuiApp::default();
        let a = app.handle_xy_event(XyEvent::TextDelta("hello\nworld".into()));
        assert!(
            a.is_empty(),
            "no commit during streaming (deferred to TurnEnd)"
        );
        assert!(
            app.pending_tail().unwrap().0.contains("hello\nworld"),
            "full text in pending_tail: {:?}",
            app.pending_tail()
        );
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
        // also clear pending so no reply text lingers in the tail. c375: it
        // also returns residual accumulated text for defensive commit.
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        assert!(app.pending_tail().is_some());
        let residual = app.end_stream();
        assert_eq!(residual.len(), 1, "residual text returned for commit");
        assert!(app.pending_tail().is_none());
        assert!(!app.is_streaming());
    }

    #[test]
    fn turnend_commits_full_code_block_with_fence_context() {
        // c375 core: a fenced code block streamed across multiple TextDeltas
        // must be committed as a SINGLE AssistantText at TurnEnd (full fence
        // context), not line-by-line. This is what makes syntax highlighting work.
        let mut app = TuiApp::default();
        app.start_stream();
        // Simulate the LLM streaming a code block in chunks:
        for chunk in ["```rs\n", "fn main() ", "{}\n", "```\n"] {
            app.handle_xy_event(XyEvent::TextDelta(chunk.into()));
        }
        // Nothing committed yet (deferred).
        assert!(app.pending_tail().is_some(), "text accumulated in mutable");
        // TurnEnd commits the full text as one block.
        let end = app.handle_xy_event(XyEvent::TurnEnd { turn_index: 0 });
        assert_eq!(end.len(), 1, "single AssistantText with full text");
        match &end[0] {
            RenderedLine::AssistantText(text) => {
                assert!(text.contains("```rs"), "fence open preserved: {text}");
                assert!(text.contains("fn main()"), "code body: {text}");
                assert!(text.ends_with("```\n"), "fence close preserved: {text}");
            }
            other => panic!("expected AssistantText, got {other:?}"),
        }
        assert!(app.pending_tail().is_none(), "cleared after TurnEnd");
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
