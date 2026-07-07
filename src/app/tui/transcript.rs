//! Transcript widget — the host-owned conversation surface (c399 stage 4).
//!
//! The pi-tui line-array engine owns the render tree root, but the skill is
//! explicit that the host app owns the "transcript surface" (the growing
//! conversation history) on top of the engine + widgets
//! (`.agents/skills/tui-pro-of-pi-tui/references/ux.md` Step "host integration").
//! The engine exposes no `root_mut` (its `root: Box<dyn Component>` is a trait
//! object with no downcast path to `Container::add`), so the host keeps a
//! handle and mutates it directly, then calls `request_render`.
//!
//! This widget is that surface: a flat `Vec<StyledLine>` for finalized
//! scrollback + an optional `Vec<StyledLine>` "pending tail" for the currently
//! streaming (unterminated) reply. `render(width)` concatenates them; the
//! engine diffs the result against the previous frame and writes only changed
//! lines. The host holds `Rc<RefCell<TranscriptWidget>>`, mutates it on
//! XyEvent/Tick, and lets the engine borrow it on render.

use crate::app::tui::engine::component::Component;
use crate::app::tui::engine::style::StyledLine;

/// The conversation surface. Finalized lines grow monotonically (one
/// [`StyledLine`] per physical row); `pending` holds the streaming mutable
/// tail, replaced wholesale each frame (the engine's diff writes only the
/// changed trailing rows).
#[derive(Default)]
pub struct TranscriptWidget {
    /// Finalized history rows (committed on paragraph/turn boundaries).
    lines: Vec<StyledLine>,
    /// Currently-streaming rows appended after `lines` each frame. Cleared on
    /// turn end. May be empty (nothing pending).
    pending: Vec<StyledLine>,
}

impl TranscriptWidget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append finalized rows to the scrollback. Called when a paragraph/turn
    /// boundary produces complete lines.
    pub fn commit(&mut self, rows: Vec<StyledLine>) {
        self.lines.extend(rows);
    }

    /// Replace the streaming pending tail wholesale. The host computes the
    /// current pending rows (wrapping the live buffer to width) and hands them
    /// here each frame it has new streaming data.
    pub fn set_pending(&mut self, rows: Vec<StyledLine>) {
        self.pending = rows;
    }

    /// Clear the pending tail (turn ended / aborted).
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    /// Total finalized row count (excludes pending). Diagnostic accessor.
    #[allow(dead_code)]
    pub fn finalized_len(&self) -> usize {
        self.lines.len()
    }
}

impl Component for TranscriptWidget {
    fn render(&self, _width: usize) -> Vec<StyledLine> {
        // The host pre-wraps both finalized and pending rows to the current
        // width before stashing them, so render is a straight concatenation.
        // (Width is passed for the trait contract; re-wrapping here would
        // double-wrap. If a width change invalidates cached wraps, the host
        // re-commits on resize — see the host loop's resize handling.)
        let mut out = Vec::with_capacity(self.lines.len() + self.pending.len());
        out.extend(self.lines.iter().cloned());
        out.extend(self.pending.iter().cloned());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::engine::style::{CellStyle, Span};

    fn raw(text: &str) -> StyledLine {
        let mut l = StyledLine::new();
        l.spans
            .push(Span::styled(text.to_string(), CellStyle::default()));
        l
    }

    #[test]
    fn empty_renders_nothing() {
        let t = TranscriptWidget::new();
        assert!(t.render(80).is_empty());
    }

    #[test]
    fn commit_grows_lines() {
        let mut t = TranscriptWidget::new();
        t.commit(vec![raw("a"), raw("b")]);
        let r = t.render(80);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].plain_text(), "a");
    }

    #[test]
    fn pending_appended_after_finalized() {
        let mut t = TranscriptWidget::new();
        t.commit(vec![raw("done")]);
        t.set_pending(vec![raw("streaming")]);
        let r = t.render(80);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].plain_text(), "done");
        assert_eq!(r[1].plain_text(), "streaming");
    }

    #[test]
    fn clear_pending_keeps_finalized() {
        let mut t = TranscriptWidget::new();
        t.commit(vec![raw("done")]);
        t.set_pending(vec![raw("streaming")]);
        t.clear_pending();
        let r = t.render(80);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].plain_text(), "done");
    }

    #[test]
    fn set_pending_overwrites_previous_pending() {
        let mut t = TranscriptWidget::new();
        t.set_pending(vec![raw("old")]);
        t.set_pending(vec![raw("new")]);
        let r = t.render(80);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].plain_text(), "new");
    }
}
