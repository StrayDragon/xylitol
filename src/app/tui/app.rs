//! Streaming state for the TUI host loop (c399 stage 4: ratatui-free).
//!
//! Holds the mutable streaming state the host loop reads: the current streaming
//! assistant/thinking text (newline-gated buffers), tool status, and whether a
//! turn is active. The host loop (in `mod.rs`) drains the agent's [`XyEvent`]
//! stream via `spawn_drain` and applies events via [`TuiApp::handle_xy_event`].
//!
//! c399 stage 4 removed the input-buffer fields and methods from this struct:
//! input is now owned by the [`crate::app::tui::widgets::input::Input`] widget
//! (held by the host as `Rc<RefCell<Input>>`), not here. What remains is purely
//! the streaming/reply accumulation concern.

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::core::driver::EventStream;
use crate::app::tui::render::{RenderedLine, xyevent_to_rendered};
use crate::domain::lifecycle::XyEvent;

/// What the mutable line is currently showing, and how to style it.
/// Read by the host loop to render the streaming pending tail.
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
// scrollback）；未换行的尾部是 mutable last line（每帧重绘，视觉上紧贴
// scrollback 生长）。对标 codex MarkdownStreamCollector 的纯文本极简版——无 markdown
// 解析、无 table holdback、无动画 chunking。

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
    ///   drain — each newline-terminated line commits immediately.
    /// - **Inside a code fence** (opened by a ``` line): lines accumulate
    ///   until the matching closing ``` line, then the whole block commits
    ///   as ONE unit so render_markdown gets complete fence context.
    ///
    /// Only fully newline-terminated content is considered; the un-terminated
    /// tail stays for the mutable region (pending_tail).
    pub fn drain_complete_paragraphs(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        let scan_end = match self.buffer[self.committed_len..].rfind('\n') {
            Some(rel) => self.committed_len + rel + 1,
            None => return out, // no complete line yet
        };
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
                local_in_fence = true;
            } else if is_fence && local_in_fence {
                local_in_fence = false;
                let block = self.buffer[self.committed_len..line_end + 1]
                    .trim_end()
                    .to_string();
                out.push(block);
                self.committed_len = line_end + 1;
            } else if !local_in_fence {
                if !line.trim().is_empty() {
                    out.push(line);
                }
                self.committed_len = line_end + 1;
            }
            i = line_end + 1;
        }
        self.in_fence = local_in_fence;
        out
    }

    /// Drain all complete lines (legacy, c365).
    pub fn drain_complete_lines(&mut self) -> Vec<String> {
        self.drain_complete_paragraphs()
    }

    /// The un-terminated tail (mutable last line content). Redrawn each frame.
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

/// The live streaming state (no input buffer — that lives in the Input widget).
#[derive(Default)]
pub struct TuiApp {
    /// Newline-gated streaming buffer for main reply text (TextDelta).
    stream_buf: StreamBuffer,
    /// Newline-gated streaming buffer for reasoning text (ThinkingDelta).
    thinking_buf: StreamBuffer,
    streaming: bool,
    /// True while in the thinking phase (before the first TextDelta). The
    /// mutable line shows thinking content (or a `Thinking…` placeholder).
    thinking_phase: bool,
    /// Active tool execution status label, shown as the mutable line when no
    /// streaming text/thinking is pending.
    tool_status: Option<String>,
}

impl TuiApp {
    // ── streaming lifecycle ─────────────────────────────────────
    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    /// Mark a turn active. The caller spawns the drain task that forwards
    /// `XyEvent`s into the shared `Msg` channel.
    pub fn start_stream(&mut self) {
        self.streaming = true;
        self.thinking_phase = true;
        self.stream_buf = StreamBuffer::default();
        self.thinking_buf = StreamBuffer::default();
        self.tool_status = None;
    }

    /// Spawn a task draining `stream` into `tx`, one `XyEvent` per message,
    /// until the stream ends (`None`) or cancellation. The drain runs to
    /// stream end: a multi-round tool-calling turn emits one `TurnEnd` per
    /// ReAct iteration, and only the final stream `None` marks the whole
    /// user turn done (c370).
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
    /// here — it lives in [`pending_tail`](Self::pending_tail) and is rendered
    /// by the host as the pending region each frame.
    ///
    /// Business state updates (pending accumulation, status line) stay here;
    /// the XyEvent→UI-data translation goes through the `xyevent_to_rendered`
    /// seam (spec tui42).
    pub fn handle_xy_event(&mut self, event: XyEvent) -> Vec<RenderedLine> {
        let mut rendered = xyevent_to_rendered(&event);
        match &event {
            XyEvent::ThinkingDelta(text) => {
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
    pub fn end_stream(&mut self) {
        self.streaming = false;
        self.thinking_phase = false;
        self.tool_status = None;
        self.stream_buf = StreamBuffer::default();
        self.thinking_buf = StreamBuffer::default();
    }

    /// The mutable line content + its kind. Redrawn each frame as the pending
    /// tail. Returns `None` when there is nothing to show.
    ///
    /// While in the thinking phase with an empty thinking buffer, returns the
    /// `Thinking…` placeholder (gray) — this is the activity indicator.
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
        assert!(app.pending_tail().is_none());
    }

    #[test]
    fn end_stream_clears_pending() {
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
        let _ = buf.drain_complete_lines();
        assert_eq!(buf.finalize(), Some("b".to_string()));
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
        let mut buf = StreamBuffer::default();
        buf.push("```rs\nfn main() {}\n");
        let mid = buf.drain_complete_paragraphs();
        assert!(mid.is_empty(), "no commit while fence unclosed: {mid:?}");
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
        let mut buf = StreamBuffer::default();
        buf.push("hello\nworld\n");
        let out = buf.drain_complete_paragraphs();
        assert_eq!(out, vec!["hello".to_string(), "world".to_string()]);
    }

    // ── Multi-round tool-turn drain (c370) ──────────────────────────

    fn event_stream(events: Vec<XyEvent>) -> EventStream {
        Box::pin(futures::stream::iter(events))
    }

    #[tokio::test]
    async fn drain_runs_past_intermediate_turnend_to_stream_end() {
        let events = vec![
            XyEvent::TextDelta("let me check\n".into()),
            XyEvent::ToolExecutionEnd {
                id: "t1".into(),
                name: "bash".into(),
                result: "ok".into(),
                is_error: false,
            },
            XyEvent::TurnEnd { turn_index: 0 },
            XyEvent::TextDelta("the answer is 42\n".into()),
            XyEvent::TurnEnd { turn_index: 1 },
        ];
        let stream = event_stream(events.clone());
        let (tx, mut rx) = mpsc::unbounded_channel::<XyEvent>();
        let cancel = Arc::new(CancellationToken::new());
        TuiApp::spawn_drain(stream, tx, cancel);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let mut received = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            received.push(ev);
        }
        assert_eq!(received.len(), events.len());
        assert!(received.iter().any(|ev| matches!(
            ev,
            XyEvent::TextDelta(t) if t.contains("the answer is 42")
        )));
    }

    #[tokio::test]
    async fn drain_stops_on_cancel() {
        let stream = event_stream(vec![XyEvent::TextDelta("maybe\n".into())]);
        let (tx, mut rx) = mpsc::unbounded_channel::<XyEvent>();
        let cancel = Arc::new(CancellationToken::new());
        cancel.cancel();
        TuiApp::spawn_drain(stream, tx, cancel);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
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
        let stale = Arc::new(CancellationToken::new());
        stale.cancel();
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
        assert_eq!(received.len(), 2);
        let _ = stale;
    }
}
