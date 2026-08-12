//! Application-owned scroll viewport for ApplicationOwned sessions (c2070).

/// Content-backed viewport: scroll is owned by the app, not the terminal scrollback.
#[derive(Debug, Clone, Default)]
pub struct ScrollView {
    lines: Vec<String>,
    /// Index of the first visible content line.
    scroll_top: usize,
    /// Visible row count (transcript pane height).
    viewport_height: usize,
}

impl ScrollView {
    /// Shared motion quantum for wheel notches and selection edge-drag.
    /// Keeps ApplicationOwned browse / select scroll feeling the same.
    pub fn motion_step(viewport_height: usize) -> isize {
        (viewport_height as isize / 8).clamp(2, 8)
    }

    pub fn new(viewport_height: usize) -> Self {
        Self {
            lines: Vec::new(),
            scroll_top: 0,
            viewport_height: viewport_height.max(1),
        }
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height.max(1);
        self.clamp_scroll();
    }

    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
        self.clamp_scroll();
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn scroll_top(&self) -> usize {
        self.scroll_top
    }

    pub fn viewport_height(&self) -> usize {
        self.viewport_height
    }

    pub fn content_len(&self) -> usize {
        self.lines.len()
    }

    pub fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(self.viewport_height)
    }

    pub fn scroll_by(&mut self, delta: isize) -> bool {
        let before = self.scroll_top;
        if delta > 0 {
            self.scroll_top = self
                .scroll_top
                .saturating_add(delta as usize)
                .min(self.max_scroll());
        } else if delta < 0 {
            self.scroll_top = self.scroll_top.saturating_sub((-delta) as usize);
        }
        self.scroll_top != before
    }

    pub fn scroll_to_end(&mut self) {
        self.scroll_top = self.max_scroll();
    }

    pub fn scroll_to_start(&mut self) {
        self.scroll_top = 0;
    }

    /// True when the viewport already shows the last content page.
    pub fn at_bottom(&self) -> bool {
        self.scroll_top >= self.max_scroll()
    }

    /// Visible content lines for the current scroll window (length ≤ viewport_height).
    pub fn visible_lines(&self) -> &[String] {
        let start = self.scroll_top.min(self.lines.len());
        let end = (start + self.viewport_height).min(self.lines.len());
        &self.lines[start..end]
    }

    /// Map a screen row within the transcript pane (0 = top of pane) to a content
    /// line index, or `None` if outside content.
    pub fn content_row_at_screen(&self, screen_row: usize) -> Option<usize> {
        if screen_row >= self.viewport_height {
            return None;
        }
        let idx = self.scroll_top + screen_row;
        (idx < self.lines.len()).then_some(idx)
    }

    fn clamp_scroll(&mut self) {
        self.scroll_top = self.scroll_top.min(self.max_scroll());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_slice_and_scroll() {
        let mut sv = ScrollView::new(2);
        sv.set_lines(vec!["a".into(), "b".into(), "c".into(), "d".into()]);
        assert_eq!(sv.visible_lines(), &["a".to_string(), "b".to_string()]);
        assert!(sv.scroll_by(1));
        assert_eq!(sv.visible_lines(), &["b".to_string(), "c".to_string()]);
        sv.scroll_to_end();
        assert_eq!(sv.scroll_top(), 2);
        assert_eq!(sv.visible_lines(), &["c".to_string(), "d".to_string()]);
        assert_eq!(sv.content_row_at_screen(0), Some(2));
        assert_eq!(sv.content_row_at_screen(2), None);
    }
}
