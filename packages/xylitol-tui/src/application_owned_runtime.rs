//! Application-owned runtime: ScrollView + selection + dock split.
//!
//! Owned by [`crate::TUI`] while an application session is active.

use std::time::{Duration, Instant};

use crate::scroll_view::ScrollView;
use crate::selection::{ClipboardSink, ScreenRect, SelectionController, format_osc52};
use crossterm::event::MouseEvent;

/// Default TTL for the ApplicationOwned copy-notice cue (ptim15 / design §7).
pub const COPY_NOTICE_TTL: Duration = Duration::from_millis(2000);

/// Per-session ApplicationOwned state (created on begin, dropped on end).
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

    /// Install / clear transcript click priority (fold hit, etc.). Prefer
    /// [`crate::TUI::set_transcript_hit_priority`] so the hook survives
    /// session recreate (suspend / begin).
    pub fn set_hit_priority(&mut self, hit: Option<crate::selection::HitPriorityFn>) {
        self.selection.set_hit_priority(hit);
    }

    /// Whether transcript selection currently spans at least one cell.
    pub fn has_selection(&self) -> bool {
        self.selection.has_selection()
    }

    /// Clear transcript selection / drag without copying.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Selected transcript plain text, if any.
    pub fn selected_text(&self) -> Option<String> {
        self.selection.selected_text(self.scroll.lines())
    }

    /// Full transcript + last dock — written to main-screen scrollback when
    /// leaving ApplicationOwned if [`crate::TUI::append_session_to_main_scrollback_on_exit`].
    pub fn session_lines_for_main_scrollback(&self) -> Vec<String> {
        let mut lines = self.scroll.lines().to_vec();
        lines.extend(self.last_dock_lines.iter().cloned());
        lines
    }

    /// True after at least one [`Self::project_frame`] ingested dock/content.
    pub fn has_projected_content(&self) -> bool {
        !self.last_dock_lines.is_empty() || self.scroll.content_len() > 0
    }

    /// Transcript content lines currently held by the scroll view (no dock).
    pub fn content_len(&self) -> usize {
        self.scroll.content_len()
    }

    /// Dock lines cached from the last component project.
    pub fn dock_line_count(&self) -> usize {
        self.last_dock_lines.len()
    }

    pub fn scroll_top(&self) -> usize {
        self.scroll.scroll_top()
    }

    /// Apply transcript scroll immediately (selection drag / tests).
    pub fn scroll_by(&mut self, delta: isize) -> bool {
        if delta == 0 {
            return false;
        }
        if !self.scroll.scroll_by(delta) {
            return false;
        }
        self.follow_bottom = self.scroll.at_bottom();
        true
    }

    /// Re-arm follow mode and move the application-owned viewport to its end.
    pub fn scroll_to_end(&mut self) -> bool {
        let was_at_bottom = self.scroll.at_bottom();
        self.scroll.scroll_to_end();
        self.follow_bottom = true;
        !was_at_bottom
    }

    /// Apply a wheel delta immediately. Hosts map each physical wheel event to
    /// [`Self::wheel_notch`] rows and may coalesce buffered events into one delta.
    pub fn ingest_wheel_delta(&mut self, delta: isize) -> bool {
        self.scroll_by(delta)
    }

    /// Wheel step for host coalescing. Edge-drag uses [`ScrollView::motion_step`].
    pub fn wheel_notch(&self) -> isize {
        ScrollView::wheel_notch()
    }

    /// Precise one-row edge-drag quantum (not used for wheel notches).
    pub fn motion_step(&self) -> isize {
        ScrollView::motion_step(self.scroll.viewport_height())
    }

    /// Split full component output into viewport paint lines (≤ terminal height).
    pub fn project_frame(&mut self, full_lines: &[String], term_height: usize) -> Vec<String> {
        let dock = self.dock_rows.min(full_lines.len()).min(term_height);
        let content_end = full_lines.len().saturating_sub(dock);
        let content = &full_lines[..content_end];
        let dock_lines = &full_lines[content_end..];
        self.last_dock_lines = dock_lines.to_vec();

        // Streaming-reflow guard: a *completed* selection goes stale once rows
        // at/above its anchor changed identity (re-wrap shifts row indices out
        // from under it). Active drags keep their coordinates — focus
        // recomputes per mouse event, and cancelling mid-drag would violate
        // the no-copy-cancel-during-drag rule (ptim12).
        if self.selection.has_selection() && !self.selection.is_dragging() {
            let top_row = self
                .selection
                .bounds()
                .map(|(start, _)| start.row)
                .unwrap_or(0);
            if Self::prefix_changed(self.scroll.lines(), content, top_row) {
                self.selection.clear();
            }
        }

        let viewport_h = term_height.saturating_sub(dock);
        self.scroll.set_viewport_height(viewport_h.max(1));
        self.scroll.set_lines(content.to_vec());
        self.paint_visible(viewport_h, term_height)
    }

    /// True when rows `[0, upto]` differ between old and new content.
    ///
    /// Appends below the anchor (`upto` prefix identical) keep the selection;
    /// any mutation at/above it invalidates the row-index coordinates.
    fn prefix_changed(old: &[String], new: &[String], upto: usize) -> bool {
        let n = upto.saturating_add(1);
        if old.len() < n || new.len() < n {
            return true;
        }
        old[..n] != new[..n]
    }

    /// Re-slice the already-ingested transcript + dock after scroll/selection
    /// changes — **no** component re-render. Used by ApplicationOwned wheel/drag.
    pub fn reproject_frame(&mut self, term_height: usize) -> Vec<String> {
        let dock = self
            .dock_rows
            .min(self.last_dock_lines.len())
            .min(term_height);
        let viewport_h = term_height.saturating_sub(dock);
        self.scroll.set_viewport_height(viewport_h.max(1));
        self.paint_visible(viewport_h, term_height)
    }

    fn paint_visible(&mut self, viewport_h: usize, term_height: usize) -> Vec<String> {
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
        visible.extend(self.last_dock_lines.iter().cloned());
        // Hard cap: never exceed terminal height.
        if visible.len() > term_height {
            visible.truncate(term_height);
        }
        visible
    }

    pub fn handle_mouse(&mut self, event: &MouseEvent, term_cols: u16, term_rows: u16) -> bool {
        use crossterm::event::MouseEventKind;

        let dock = self.dock_rows.min(term_rows as usize) as u16;
        let transcript_h = term_rows.saturating_sub(dock);
        let transcript = ScreenRect {
            row: 0,
            col: 0,
            height: transcript_h,
            width: term_cols,
        };
        // Host-side coalescing sums physical wheel steps before calling
        // `ingest_wheel_delta`, but never changes the per-event notch.
        let wheel_delta = match event.kind {
            MouseEventKind::ScrollUp => Some(-self.wheel_notch()),
            MouseEventKind::ScrollDown => Some(self.wheel_notch()),
            _ => None,
        };
        if let Some(delta) = wheel_delta {
            if !transcript.contains(event.column, event.row) {
                return false;
            }
            return self.ingest_wheel_delta(delta);
        }

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
        // Edge scroll away from bottom clears follow; return to bottom re-arms it.
        if dirty {
            self.follow_bottom = self.scroll.at_bottom();
        }
        dirty
    }

    pub fn tick_autoscroll(&mut self, term_cols: u16, term_rows: u16) -> bool {
        let dock = self.dock_rows.min(term_rows as usize) as u16;
        let transcript_h = term_rows.saturating_sub(dock);
        // Keep viewport height in sync before selection edge-drag motion.
        self.scroll
            .set_viewport_height((transcript_h as usize).max(1));
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

/// Test helper: construct an ApplicationOwned runtime (selection/copy covered by selection tests).
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
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let paint = runtime.project_frame(&full, 8);
        assert_eq!(paint.len(), 8);
        assert_eq!(paint[6], "L18");
        assert_eq!(paint[7], "L19");
        // Visible content is last 6 of 18 content lines.
        assert_eq!(paint[0], "L12");
    }

    #[test]
    fn wheel_scroll_persists_across_project_frame() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let _ = runtime.project_frame(&full, 8);
        assert!(runtime.scroll.at_bottom());
        let top_before = runtime.scroll.scroll_top();
        let notch = runtime.wheel_notch();
        let wheel = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        assert!(runtime.handle_mouse(&wheel, 40, 8));
        // One physical notch is applied immediately.
        assert_eq!(runtime.scroll.scroll_top(), top_before - notch as usize);
        let top_scrolled = runtime.scroll.scroll_top();
        let paint = runtime.project_frame(&full, 8);
        assert_eq!(runtime.scroll.scroll_top(), top_scrolled);
        assert_eq!(paint[0], format!("L{top_scrolled}"));
    }

    #[test]
    fn wheel_coalesce_applies_full_delta_immediately() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..40).map(|i| format!("L{i}")).collect();
        let _ = runtime.project_frame(&full, 10);
        let notch = runtime.wheel_notch();
        assert_eq!(notch, 3);
        let top0 = runtime.scroll.scroll_top();
        assert!(runtime.ingest_wheel_delta(-(notch * 3)));
        assert_eq!(runtime.scroll.scroll_top(), top0 - (notch * 3) as usize);
        assert!(
            !runtime.tick_autoscroll(40, 10),
            "normal wheel motion must not drip through the selection tick path"
        );
        assert_eq!(runtime.scroll.scroll_top(), top0 - (notch * 3) as usize);
    }

    #[test]
    fn scroll_to_end_rearms_follow_mode() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let _ = runtime.project_frame(&full, 8);
        assert!(runtime.scroll_by(-runtime.wheel_notch()));
        assert!(!runtime.follow_bottom);

        assert!(runtime.scroll_to_end());
        assert!(runtime.scroll.at_bottom());
        assert!(runtime.follow_bottom);
        assert!(!runtime.scroll_to_end());
    }

    #[test]
    fn dock_that_fills_terminal_keeps_dock_and_disables_transcript_selection() {
        let mut runtime = ApplicationOwnedRuntime::new(8);
        let full: Vec<String> = (0..8).map(|i| format!("L{i}")).collect();
        let paint = runtime.project_frame(&full, 4);
        assert_eq!(paint, vec!["L4", "L5", "L6", "L7"]);

        assert!(!runtime.handle_mouse(
            &mouse(MouseEventKind::Down(MouseButton::Left), 0, 0),
            40,
            4,
        ));
        assert!(!runtime.selection.is_dragging());
    }

    #[test]
    fn copy_on_release_signals_notice_empty_does_not() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        let full: Vec<String> = vec![
            "hello world".into(),
            "second".into(),
            "status".into(),
            "input".into(),
        ];
        let _ = runtime.project_frame(&full, 6);
        // Empty click — no notice.
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 1, 0), 40, 6));
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 1, 0), 40, 6));
        assert!(!runtime.copy_notice_active());
        assert!(!runtime.take_copy_notice());

        // Drag select then release — notice armed.
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 0), 40, 6));
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Drag(MouseButton::Left), 5, 0), 40, 6));
        assert!(runtime.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 5, 0), 40, 6));
        assert!(runtime.copy_notice_active());
        assert!(runtime.take_copy_notice());
        assert!(!runtime.take_copy_notice());
        assert!(runtime.copy_notice_active());
        assert!(!runtime.take_pending_clipboard().is_empty());
    }

    #[test]
    fn copy_disabled_does_not_signal_notice() {
        let mut runtime = ApplicationOwnedRuntime::new(2);
        runtime.set_copy_on_release(false);
        let full: Vec<String> = vec!["abcd".into(), "e".into(), "s".into(), "i".into()];
        let _ = runtime.project_frame(&full, 6);
        runtime.handle_mouse(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 0), 40, 6);
        runtime.handle_mouse(&mouse(MouseEventKind::Up(MouseButton::Left), 3, 0), 40, 6);
        assert!(!runtime.copy_notice_active());
        assert!(!runtime.take_copy_notice());
        assert!(runtime.take_pending_clipboard().is_empty());
    }

    #[test]
    fn copy_notice_expires_on_tick() {
        let mut runtime = ApplicationOwnedRuntime::new(1);
        runtime.signal_copy_notice();
        // Force an already-expired deadline.
        runtime.copy_notice_until = Some(Instant::now() - Duration::from_millis(1));
        assert!(!runtime.copy_notice_active());
        assert!(runtime.tick_copy_notice());
        assert!(!runtime.take_copy_notice());
        assert!(!runtime.tick_copy_notice());
    }

    fn drag_select_rows_1_to_2(runtime: &mut ApplicationOwnedRuntime) {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let ev = |kind, col: u16, row: u16| MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        };
        assert!(runtime.handle_mouse(&ev(MouseEventKind::Down(MouseButton::Left), 0, 1), 20, 6));
        assert!(runtime.handle_mouse(&ev(MouseEventKind::Drag(MouseButton::Left), 2, 2), 20, 6));
        assert!(runtime.handle_mouse(&ev(MouseEventKind::Up(MouseButton::Left), 2, 2), 20, 6));
    }

    fn reflow_fixture(tail: &[&str], first_row: &str) -> Vec<String> {
        let mut v: Vec<String> = ["alpha", "beta", "gamma", "delta", "echo"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if first_row != "alpha" {
            v[0] = first_row.to_string();
        }
        v.extend(tail.iter().map(|s| s.to_string()));
        v.push("dock".into());
        v
    }

    #[test]
    fn completed_selection_survives_append_below_and_clears_on_reflow_above() {
        let mut runtime = ApplicationOwnedRuntime::new(1);
        let full1 = reflow_fixture(&[], "alpha");
        let _ = runtime.project_frame(&full1, 6);
        drag_select_rows_1_to_2(&mut runtime);
        assert!(runtime.has_selection(), "drag must complete with selection");

        // Streaming appends below the selection — rows ≤ anchor unchanged → kept.
        let full2 = reflow_fixture(&["foxtrot"], "alpha");
        let _ = runtime.project_frame(&full2, 7);
        assert!(runtime.has_selection(), "append below must not clear");

        // Re-wrap above the anchor — row indices shifted → stale, cleared.
        let full3 = reflow_fixture(&["foxtrot"], "alpha wraps differently now");
        let _ = runtime.project_frame(&full3, 7);
        assert!(!runtime.has_selection(), "reflow above must invalidate");
    }

    #[test]
    fn active_drag_is_not_cancelled_by_reflow_above() {
        use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let mut runtime = ApplicationOwnedRuntime::new(1);
        let full1 = reflow_fixture(&[], "alpha");
        let _ = runtime.project_frame(&full1, 6);
        let ev = |kind, col: u16, row: u16| MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        };
        assert!(runtime.handle_mouse(&ev(MouseEventKind::Down(MouseButton::Left), 0, 1), 20, 6));
        assert!(runtime.selection.is_dragging());

        // Mid-drag frame with changed rows above: keep dragging (ptim12).
        let full2 = reflow_fixture(&[], "alpha wraps differently now");
        let _ = runtime.project_frame(&full2, 6);
        assert!(runtime.selection.is_dragging());
    }
}
