//! Mode B (application-owned) runtime: ScrollView + selection + dock split.
//!
//! Owned by [`crate::TUI`] while an application session is active.

use std::time::{Duration, Instant};

use crate::scroll_view::ScrollView;
use crate::selection::{ClipboardSink, ScreenRect, SelectionController, format_osc52};
use crossterm::event::MouseEvent;

/// Default TTL for the Mode B copy-notice cue (ptim15 / design §7).
pub const COPY_NOTICE_TTL: Duration = Duration::from_millis(2000);

/// Per-session Mode B state (created on begin, dropped on end).
pub struct ApplicationOwnedRuntime {
    pub scroll: ScrollView,
    pub selection: SelectionController,
    /// Bottom dock rows excluded from transcript selection (editor/status/footer…).
    pub dock_rows: usize,
    /// OSC52 (or other) sequences to flush **outside** a differential paint batch.
    pub pending_clipboard: Vec<String>,
    /// Stick to the latest content unless the user scrolls away (wheel / edge).
    follow_bottom: bool,
    /// Last dock lines from [`Self::project_frame`] (for main-scrollback append on exit).
    last_dock_lines: Vec<String>,
    /// Edge signal: set on successful copy-on-release until [`Self::take_copy_notice`].
    pending_copy_notice: bool,
    /// Visible/active notice deadline; cleared by [`Self::tick_copy_notice`].
    copy_notice_until: Option<Instant>,
}

impl ApplicationOwnedRuntime {
    pub fn new(dock_rows: usize) -> Self {
        Self {
            scroll: ScrollView::new(1),
            selection: SelectionController::new(),
            dock_rows: dock_rows.max(1),
            pending_clipboard: Vec::new(),
            follow_bottom: true,
            last_dock_lines: Vec::new(),
            pending_copy_notice: false,
            copy_notice_until: None,
        }
    }

    pub fn set_dock_rows(&mut self, rows: usize) {
        self.dock_rows = rows.max(1);
    }

    pub fn set_copy_on_release(&mut self, on: bool) {
        self.selection.copy_on_release = on;
    }

    /// Full transcript + last dock — written to main-screen scrollback when
    /// leaving Mode B if [`crate::TUI::append_session_to_main_scrollback_on_exit`].
    pub fn session_lines_for_main_scrollback(&self) -> Vec<String> {
        let mut lines = self.scroll.lines().to_vec();
        lines.extend(self.last_dock_lines.iter().cloned());
        lines
    }

    /// Split full component output into viewport paint lines (≤ terminal height).
    pub fn project_frame(&mut self, full_lines: &[String], term_height: usize) -> Vec<String> {
        let dock = self.dock_rows.min(full_lines.len()).min(term_height);
        let content_end = full_lines.len().saturating_sub(dock);
        let content: Vec<String> = full_lines[..content_end].to_vec();
        let dock_lines: Vec<String> = full_lines[content_end..].to_vec();
        self.last_dock_lines = dock_lines.clone();

        let viewport_h = term_height.saturating_sub(dock).max(1);
        self.scroll.set_viewport_height(viewport_h);
        self.scroll.set_lines(content);
        // Follow only when sticky; wheel / selection edge scroll must persist across frames.
        if self.follow_bottom && !self.selection.is_dragging() {
            self.scroll.scroll_to_end();
        }

        let mut visible = self.scroll.visible_lines().to_vec();
        self.selection
            .apply_highlight(&mut visible, self.scroll.scroll_top());
        while visible.len() < viewport_h {
            visible.push(String::new());
        }
        visible.truncate(viewport_h);
        visible.extend(dock_lines);
        // Hard cap: never exceed terminal height.
        if visible.len() > term_height {
            visible.truncate(term_height);
        }
        visible
    }

    pub fn handle_mouse(&mut self, event: &MouseEvent, term_cols: u16, term_rows: u16) -> bool {
        let dock = self.dock_rows.min(term_rows as usize) as u16;
        let transcript_h = term_rows.saturating_sub(dock).max(1);
        let transcript = ScreenRect {
            row: 0,
            col: 0,
            height: transcript_h,
            width: term_cols,
        };
        let dock_rect = ScreenRect {
            row: transcript_h,
            col: 0,
            height: dock,
            width: term_cols,
        };
        let mut sink = CollectOsc52Sink {
            out: &mut self.pending_clipboard,
        };
        let dirty =
            self.selection
                .handle_mouse(event, &mut self.scroll, transcript, dock_rect, &mut sink);
        if self.selection.last_event_copied() {
            self.signal_copy_notice();
        }
        // Wheel / edge scroll away from bottom clears follow; return to bottom re-arms it.
        if dirty {
            self.follow_bottom = self.scroll.at_bottom();
        }
        dirty
    }

    pub fn tick_autoscroll(&mut self, term_cols: u16, term_rows: u16) -> bool {
        let dock = self.dock_rows.min(term_rows as usize) as u16;
        let transcript_h = term_rows.saturating_sub(dock).max(1);
        let transcript = ScreenRect {
            row: 0,
            col: 0,
            height: transcript_h,
            width: term_cols,
        };
        let dirty = self.selection.tick_autoscroll(&mut self.scroll, transcript);
        if dirty {
            self.follow_bottom = self.scroll.at_bottom();
        }
        dirty
    }

    /// Expire copy-notice when past deadline. Returns true if the notice cleared
    /// (caller should rerender so the cue disappears).
    pub fn tick_copy_notice(&mut self) -> bool {
        let Some(until) = self.copy_notice_until else {
            return false;
        };
        if Instant::now() < until {
            return false;
        }
        self.copy_notice_until = None;
        self.pending_copy_notice = false;
        true
    }

    /// Consume the edge signal armed by a successful copy-on-release.
    pub fn take_copy_notice(&mut self) -> bool {
        std::mem::take(&mut self.pending_copy_notice)
    }

    /// Whether the copy-notice is still within its TTL window.
    pub fn copy_notice_active(&self) -> bool {
        self.copy_notice_until
            .is_some_and(|until| Instant::now() < until)
    }

    fn signal_copy_notice(&mut self) {
        self.pending_copy_notice = true;
        self.copy_notice_until = Some(Instant::now() + COPY_NOTICE_TTL);
    }

    /// Queue OSC52 (or other) sequences from a non-transcript source (e.g. Editor)
    /// and arm copy-notice when at least one non-empty sequence is enqueued.
    pub fn enqueue_clipboard_seqs(&mut self, seqs: impl IntoIterator<Item = String>) {
        let mut any = false;
        for seq in seqs {
            if !seq.is_empty() {
                self.pending_clipboard.push(seq);
                any = true;
            }
        }
        if any {
            self.signal_copy_notice();
        }
    }

    pub fn take_pending_clipboard(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_clipboard)
    }
}

struct CollectOsc52Sink<'a> {
    out: &'a mut Vec<String>,
}

impl ClipboardSink for CollectOsc52Sink<'_> {
    fn copy_text(&mut self, text: &str) {
        if let Some(seq) = format_osc52(text) {
            self.out.push(seq);
        } else {
            let _ = text;
        }
    }
}

/// Test helper: construct a Mode B runtime (selection/copy covered by selection tests).
#[cfg(test)]
pub fn test_runtime(dock_rows: usize) -> ApplicationOwnedRuntime {
    ApplicationOwnedRuntime::new(dock_rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};

    fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn project_frame_caps_height_and_keeps_dock() {
        let mut mb = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let paint = mb.project_frame(&full, 8);
        assert_eq!(paint.len(), 8);
        assert_eq!(paint[6], "L18");
        assert_eq!(paint[7], "L19");
        // Visible content is last 6 of 18 content lines.
        assert_eq!(paint[0], "L12");
    }

    #[test]
    fn wheel_scroll_persists_across_project_frame() {
        let mut mb = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let _ = mb.project_frame(&full, 8);
        assert!(mb.scroll.at_bottom());
        let top_before = mb.scroll.scroll_top();
        let wheel = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        assert!(mb.handle_mouse(&wheel, 40, 8));
        assert!(!mb.scroll.at_bottom());
        assert!(mb.scroll.scroll_top() < top_before);
        let top_scrolled = mb.scroll.scroll_top();
        let paint = mb.project_frame(&full, 8);
        assert_eq!(mb.scroll.scroll_top(), top_scrolled);
        assert_eq!(paint[0], format!("L{top_scrolled}"));
    }

    #[test]
    fn copy_on_release_signals_notice_empty_does_not() {
        let mut mb = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = vec![
            "hello world".into(),
            "second".into(),
            "status".into(),
            "input".into(),
        ];
        let _ = mb.project_frame(&full, 6);
        // Empty click — no notice.
        assert!(mb.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 1, 0), 40, 6));
        assert!(mb.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 1, 0), 40, 6));
        assert!(!mb.copy_notice_active());
        assert!(!mb.take_copy_notice());

        // Drag select then release — notice armed.
        assert!(mb.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 0), 40, 6));
        assert!(mb.handle_mouse(&mouse(MouseEventKind::Drag(MouseButton::Left), 5, 0), 40, 6));
        assert!(mb.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 5, 0), 40, 6));
        assert!(mb.copy_notice_active());
        assert!(mb.take_copy_notice());
        assert!(!mb.take_copy_notice());
        assert!(mb.copy_notice_active());
        assert!(!mb.take_pending_clipboard().is_empty());
    }

    #[test]
    fn copy_disabled_does_not_signal_notice() {
        let mut mb = ApplicationOwnedRuntime::new(2);
        mb.set_copy_on_release(false);
        let full: Vec<String> = vec!["abcd".into(), "e".into(), "s".into(), "i".into()];
        let _ = mb.project_frame(&full, 6);
        mb.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 0), 40, 6);
        mb.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 3, 0), 40, 6);
        assert!(!mb.copy_notice_active());
        assert!(!mb.take_copy_notice());
        assert!(mb.take_pending_clipboard().is_empty());
    }

    #[test]
    fn copy_notice_expires_on_tick() {
        let mut mb = ApplicationOwnedRuntime::new(1);
        mb.signal_copy_notice();
        // Force an already-expired deadline.
        mb.copy_notice_until = Some(Instant::now() - Duration::from_millis(1));
        assert!(!mb.copy_notice_active());
        assert!(mb.tick_copy_notice());
        assert!(!mb.take_copy_notice());
        assert!(!mb.tick_copy_notice());
    }
}
