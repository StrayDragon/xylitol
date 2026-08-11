//! Mode B (application-owned) runtime: ScrollView + selection + dock split.
//!
//! Owned by [`crate::TUI`] while an application session is active.

use crate::scroll_view::ScrollView;
use crate::selection::{ClipboardSink, ScreenRect, SelectionController, format_osc52};
use crossterm::event::MouseEvent;

/// Per-session Mode B state (created on begin, dropped on end).
pub struct ModeBRuntime {
    pub scroll: ScrollView,
    pub selection: SelectionController,
    /// Bottom dock rows excluded from transcript selection (editor/status/footer…).
    pub dock_rows: usize,
    /// OSC52 (or other) sequences to flush **outside** a differential paint batch.
    pub pending_clipboard: Vec<String>,
}

impl ModeBRuntime {
    pub fn new(dock_rows: usize) -> Self {
        Self {
            scroll: ScrollView::new(1),
            selection: SelectionController::new(),
            dock_rows: dock_rows.max(1),
            pending_clipboard: Vec::new(),
        }
    }

    pub fn set_dock_rows(&mut self, rows: usize) {
        self.dock_rows = rows.max(1);
    }

    pub fn set_copy_on_release(&mut self, on: bool) {
        self.selection.copy_on_release = on;
    }

    /// Split full component output into viewport paint lines (≤ terminal height).
    pub fn project_frame(&mut self, full_lines: &[String], term_height: usize) -> Vec<String> {
        let dock = self.dock_rows.min(full_lines.len()).min(term_height);
        let content_end = full_lines.len().saturating_sub(dock);
        let content: Vec<String> = full_lines[..content_end].to_vec();
        let dock_lines: Vec<String> = full_lines[content_end..].to_vec();

        let viewport_h = term_height.saturating_sub(dock).max(1);
        self.scroll.set_viewport_height(viewport_h);
        // Follow bottom when not actively selecting (chat TUI default).
        let follow = !self.selection.is_dragging() && !self.selection.has_selection();
        self.scroll.set_lines(content);
        if follow {
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
        self.selection
            .handle_mouse(event, &mut self.scroll, transcript, dock_rect, &mut sink)
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
        self.selection.tick_autoscroll(&mut self.scroll, transcript)
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
            // Oversize: still record plain via a no-op marker for tests using Recording…
            let _ = text;
        }
    }
}

/// Test helper: construct a Mode B runtime (selection/copy covered by selection tests).
#[cfg(test)]
pub fn test_runtime(dock_rows: usize) -> ModeBRuntime {
    ModeBRuntime::new(dock_rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_frame_caps_height_and_keeps_dock() {
        let mut mb = ModeBRuntime::new(2);
        let full: Vec<String> = (0..20).map(|i| format!("L{i}")).collect();
        let paint = mb.project_frame(&full, 8);
        assert_eq!(paint.len(), 8);
        assert_eq!(paint[6], "L18");
        assert_eq!(paint[7], "L19");
        // Visible content is last 6 of 18 content lines.
        assert_eq!(paint[0], "L12");
    }
}
