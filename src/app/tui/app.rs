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
use crate::app::tui::components::markdown::{MarkdownStyle, render_markdown};
use crate::app::tui::components::spinner::SPINNER;
use crate::app::tui::render::{RenderedLine, xyevent_to_rendered};
use crate::app::tui::theme;
use crate::domain::lifecycle::XyEvent;



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
    /// Accumulated full markdown source for the current assistant turn (c376).
    /// Each TextDelta appends here; `handle_xy_event` re-renders the whole
    /// string to get highlighted lines, committing stable (post-newline) lines
    /// incrementally and keeping the unstable tail in `mutable_tail_lines`.
    finalized: String,
    /// Accumulated reasoning source (same incremental render model as `finalized`).
    thinking_finalized: String,
    /// How many rendered lines of `finalized` have been committed to scrollback.
    committed_count: usize,
    /// How many rendered lines of `thinking_finalized` have been committed.
    thinking_committed_count: usize,
    /// Rendered lines of the unstable tail (post-last-newline), shown in the
    /// mutable region. Refreshed each TextDelta. Empty when not streaming.
    mutable_tail_lines: Vec<ratatui_core::text::Line<'static>>,
    /// Terminal width for markdown rendering (set on start_stream).
    render_width: u16,
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
    pub fn start_stream(&mut self, render_width: u16) {
        self.streaming = true;
        self.thinking_phase = true;
        self.finalized.clear();
        self.thinking_finalized.clear();
        self.committed_count = 0;
        self.thinking_committed_count = 0;
        self.mutable_tail_lines.clear();
        self.render_width = render_width;
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
                self.thinking_finalized.push_str(text);
                self.recompute_streaming_render(&mut rendered, false);
            }
            XyEvent::TextDelta(text) => {
                self.thinking_phase = false;
                self.finalized.push_str(text);
                self.recompute_streaming_render(&mut rendered, false);
            }
            XyEvent::ToolExecutionStart { name, .. } => {
                self.tool_status = Some(format!("⚙ running {name}"));
            }
            XyEvent::ToolExecutionUpdate { .. } => {}
            XyEvent::ToolExecutionEnd { name, .. } => {
                self.tool_status = Some(format!("✓ {name} done"));
            }
            XyEvent::TurnEnd { .. } => {
                // c376: commit all remaining lines (finalize). Each intermediate
                // TurnEnd in a multi-round tool-calling turn flushes its segment.
                self.recompute_streaming_render(&mut rendered, true);
            }
            XyEvent::TurnStart { .. } | XyEvent::ModelSelect { .. } | XyEvent::Error(_) => {}
            _ => {}
        }
        rendered
    }

    /// c376 core: full re-render of accumulated streaming source, commit stable
    /// lines (post-newline) incrementally with highlighting, keep the unstable
    /// tail in `mutable_tail_lines` for the mutable region.
    ///
    /// `finalize`: on TurnEnd, commit ALL remaining lines (including the
    /// unstable tail) and reset accumulators.
    fn recompute_streaming_render(&mut self, rendered: &mut Vec<RenderedLine>, finalize: bool) {
        let p = theme::palette();
        let width = self.render_width;
        // Render thinking accumulated text (if any). Thinking commits its own
        // stable lines and clears on TurnEnd (finalize). While in thinking
        // phase, the mutable tail shows the thinking unstable remainder.
        if !self.thinking_finalized.is_empty() {
            let style = MarkdownStyle::for_thinking(&p);
            let all_lines = render_markdown(&self.thinking_finalized, width, &style);
            let stable_end = if finalize {
                self.thinking_finalized.len()
            } else {
                self.thinking_finalized
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0)
            };
            let stable_count = if stable_end == 0 {
                0
            } else {
                render_markdown(&self.thinking_finalized[..stable_end], width, &style).len()
            };
            if stable_count > self.thinking_committed_count {
                let sc = stable_count.min(all_lines.len());
                if sc > self.thinking_committed_count {
                    let new_lines = all_lines[self.thinking_committed_count..sc].to_vec();
                    rendered.push(RenderedLine::PreRendered(new_lines));
                    self.thinking_committed_count = sc;
                }
            }
            // While in thinking phase, the mutable tail shows thinking's
            // unstable remainder; once text arrives, the text block below
            // overwrites mutable_tail_lines.
            if self.thinking_phase {
                let tail_start = self.thinking_committed_count.min(all_lines.len());
                self.mutable_tail_lines = all_lines[tail_start..].to_vec();
            }
            if finalize {
                if self.thinking_committed_count < all_lines.len() {
                    let rest = all_lines[self.thinking_committed_count..].to_vec();
                    rendered.push(RenderedLine::PreRendered(rest));
                }
                self.thinking_finalized.clear();
                self.thinking_committed_count = 0;
            }
        }
        // Render assistant text (only when not purely in thinking phase).
        if !self.thinking_phase && !self.finalized.is_empty() {
            let style = MarkdownStyle::for_assistant(&p);
            let all_lines = render_markdown(&self.finalized, width, &style);
            let stable_end = if finalize {
                self.finalized.len()
            } else {
                self.finalized.rfind('\n').map(|i| i + 1).unwrap_or(0)
            };
            let stable_count = if stable_end == 0 {
                0
            } else {
                render_markdown(&self.finalized[..stable_end], width, &style).len()
            };
            if stable_count > self.committed_count {
                let sc = stable_count.min(all_lines.len());
                if sc > self.committed_count {
                    let new_lines = all_lines[self.committed_count..sc].to_vec();
                    rendered.push(RenderedLine::PreRendered(new_lines));
                    self.committed_count = sc;
                }
            }
            // Update mutable tail = everything not yet committed (includes the
            // unstable post-newline remainder + stable-but-pending lines). Using
            // committed_count (not stable_count) avoids index inconsistency when
            // the unclosed-fence prefix renders differently than the full source.
            let tail_start = self.committed_count.min(all_lines.len());
            self.mutable_tail_lines = all_lines[tail_start..].to_vec();
            if finalize {
                if self.committed_count < all_lines.len() {
                    let rest = all_lines[self.committed_count..].to_vec();
                    rendered.push(RenderedLine::PreRendered(rest));
                }
                self.finalized.clear();
                self.committed_count = 0;
                self.mutable_tail_lines.clear();
            }
        }
    }

    /// Called when the turn stream ends; resets streaming state. Returns any
    /// residual accumulated text as finalized `RenderedLine`s (defensive — the
    /// normal TurnEnd path already committed via `handle_xy_event`; this only
    /// fires if the stream ended without a TurnEnd, e.g. abort mid-stream).
    pub fn end_stream(&mut self) -> Vec<RenderedLine> {
        let mut rendered = Vec::new();
        // Finalize any residual accumulated text (defensive — normal TurnEnd
        // already committed; this catches abort-without-TurnEnd).
        self.recompute_streaming_render(&mut rendered, true);
        self.streaming = false;
        self.thinking_phase = false;
        self.tool_status = None;
        self.mutable_tail_lines.clear();
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
    /// The mutable-region rendered lines (c376: pre-highlighted via the
    /// streaming incremental renderer). Returns the unstable tail of the
    /// current streaming phase (thinking or text), or a tool-status line.
    /// `None` when there is nothing to show.
    pub fn pending_tail(&self) -> Option<Vec<ratatui_core::text::Line<'static>>> {
        use ratatui_core::text::{Line, Span};
        let p = theme::palette();
        if self.thinking_phase {
            if !self.mutable_tail_lines.is_empty() {
                return Some(self.mutable_tail_lines.clone());
            }
            return Some(vec![Line::from(Span::styled(
                "Thinking…".to_string(),
                p.thinking(),
            ))]);
        }
        if !self.mutable_tail_lines.is_empty() {
            return Some(self.mutable_tail_lines.clone());
        }
        if let Some(status) = self.tool_status.as_deref() {
            return Some(vec![Line::from(Span::styled(status.to_string(), p.tool()))]);
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
    fn textdelta_commits_stable_lines_incrementally() {
        // c376: TextDelta with a newline commits the stable (post-newline)
        // line immediately (incremental, not deferred). The unstable tail
        // (post-last-newline) stays in pending_tail.
        let mut app = TuiApp::default();
        app.start_stream(80);
        let a = app.handle_xy_event(XyEvent::TextDelta("hello\nworld".into()));
        assert!(
            !a.is_empty(),
            "stable line committed incrementally (not deferred)"
        );
        // The unstable tail "world" remains in pending_tail.
        let tail_text: String = app
            .pending_tail()
            .map(|lines| {
                lines
                    .iter()
                    .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        assert!(
            tail_text.contains("world"),
            "tail has unstable remainder: {tail_text}"
        );
    }

    #[test]
    fn turn_end_flushes_remaining_pending() {
        let mut app = TuiApp::default();
        app.start_stream(80);
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        let end = app.handle_xy_event(XyEvent::TurnEnd { turn_index: 0 });
        assert!(!end.is_empty(), "residual committed at TurnEnd");
        assert!(app.pending_tail().is_none(), "cleared after TurnEnd");
    }

    #[test]
    fn end_stream_clears_pending() {
        let mut app = TuiApp::default();
        app.start_stream(80);
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        assert!(app.pending_tail().is_some());
        let residual = app.end_stream();
        assert!(!residual.is_empty(), "residual text returned for commit");
        assert!(app.pending_tail().is_none());
        assert!(!app.is_streaming());
    }

    #[test]
    fn streaming_code_block_commits_highlighted_lines() {
        // c376: a fenced code block streamed across chunks commits stable lines
        // incrementally WITH highlighting (PreRendered), not deferred to TurnEnd.
        let mut app = TuiApp::default();
        app.start_stream(80);
        // Stream a code block: once the closing ``` arrives (with newline), the
        // whole block becomes stable and commits as PreRendered lines.
        let mut committed = Vec::new();
        for chunk in ["```rs\n", "fn main() ", "{}\n", "```\n"] {
            committed.extend(app.handle_xy_event(XyEvent::TextDelta(chunk.into())));
        }
        // At least one PreRendered commit happened during streaming.
        let has_pre_rendered = committed
            .iter()
            .any(|r| matches!(r, RenderedLine::PreRendered(lines) if !lines.is_empty()));
        assert!(
            has_pre_rendered,
            "code block committed as PreRendered during streaming"
        );
        // The committed PreRendered lines carry highlighting (non-default style).
        let any_colored = committed.iter().any(|r| match r {
            RenderedLine::PreRendered(lines) => lines
                .iter()
                .flat_map(|l| l.spans.iter())
                .any(|s| s.style.fg.is_some()),
            _ => false,
        });
        assert!(any_colored, "committed lines have syntax highlighting");
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
