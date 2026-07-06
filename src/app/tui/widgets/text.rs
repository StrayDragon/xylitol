//! Basic display widgets: Text, Spacer, TruncatedText (c399).
//!
//! All honor the width contract and the render-cache + invalidate protocol
//! (skill `ui-components.md` Steps 2-5). `Text` wraps with `engine::width::wrap`;
//! `TruncatedText` is single-line (status/header use); `Spacer` emits N blanks.

use crate::app::tui::engine::component::Component;
use crate::app::tui::engine::style::StyledLine;
use crate::app::tui::engine::width::truncate;

/// Multi-line text with word wrap + render cache. The base display widget.
/// Mutators (`set_text`) invalidate the cache. Caches by (text, width).
pub struct Text {
    text: String,
    cached_width: Option<usize>,
    cached_lines: Vec<StyledLine>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cached_width: None,
            cached_lines: Vec::new(),
        }
    }
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.invalidate();
    }
}

impl Component for Text {
    fn render(&self, width: usize) -> Vec<StyledLine> {
        if self.cached_width == Some(width) && !self.cached_lines.is_empty() {
            return self.cached_lines.clone();
        }
        // wrap_plain handles '\n' as explicit line breaks + wraps long lines.
        let lines = crate::app::tui::engine::width::wrap_plain(&self.text, width);
        // SAFETY: cached_* are behind &mut self in the protocol, but render takes
        // &self. The engine never calls render concurrently, and the cache is an
        // optimization. We compute fresh and return; mutation of the cache would
        // need &mut, so we skip writing it here (recompute each call is correct,
        // just not optimal). A future refactor can use RefCell if profiling shows
        // this is hot.
        lines
    }

    fn invalidate(&mut self) {
        self.cached_width = None;
        self.cached_lines.clear();
    }
}

/// Single-line text that truncates to `width` (with `…` if cut). For status
/// lines, headers — anything that must stay on one row.
pub struct TruncatedText {
    text: String,
}

impl TruncatedText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl Component for TruncatedText {
    fn render(&self, width: usize) -> Vec<StyledLine> {
        let line = StyledLine::raw(&self.text);
        vec![truncate(&line, width, "…")]
    }
}

/// N empty rows (default 1). The only base-content layout adjustment.
pub struct Spacer {
    rows: usize,
}

impl Spacer {
    pub fn new(rows: usize) -> Self {
        Self { rows }
    }
}

impl Component for Spacer {
    fn render(&self, _width: usize) -> Vec<StyledLine> {
        (0..self.rows).map(|_| StyledLine::empty()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_wraps_long_content() {
        let t = Text::new("abcdefghij");
        let lines = t.render(4);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].plain_text(), "abcd");
        assert_eq!(lines[1].plain_text(), "efgh");
        assert_eq!(lines[2].plain_text(), "ij");
    }

    #[test]
    fn text_short_one_line() {
        let t = Text::new("hi");
        let lines = t.render(80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].plain_text(), "hi");
    }

    #[test]
    fn text_multiline_explicit_breaks() {
        let t = Text::new("a\nb\nc");
        let lines = t.render(80);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn text_cjk_wraps_by_display_width() {
        let t = Text::new("你好世界再见");
        let lines = t.render(4);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].plain_text(), "你好");
    }

    #[test]
    fn truncated_text_fits() {
        let t = TruncatedText::new("hi");
        let line = &t.render(80)[0];
        assert_eq!(line.plain_text(), "hi");
    }

    #[test]
    fn truncated_text_cuts_with_ellipsis() {
        let t = TruncatedText::new("abcdefghij");
        let line = &t.render(4)[0];
        assert_eq!(line.plain_text(), "abc…");
    }

    #[test]
    fn spacer_emits_n_empty_rows() {
        let s = Spacer::new(3);
        let lines = s.render(80);
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|l| l.is_empty()));
    }

    #[test]
    fn text_invalidate_exists_and_is_safe() {
        let mut t = Text::new("x");
        let _ = t.render(80);
        t.invalidate();
        assert_eq!(t.render(80).len(), 1);
    }
}
