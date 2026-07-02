//! `Spinner` widget — a single braille-dots glyph that cycles through frames.
//! This is a low-level reusable widget: it renders one glyph (no label, no bg).
//! Combine with a label elsewhere for a richer status line.
//!
//! Not currently wired into the TUI layout — kept as a future building block
//! (tool status rows, loading overlays, etc.).

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Span;
use ratatui_core::widgets::Widget;

use crate::app::tui::theme;

/// Braille-dots spinner frame cycle. Shared with the app's tick-driven index.
pub const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Renders a single spinner glyph at the given `area`, styled with the spinner
/// color token. The caller advances `idx` (modulo `SPINNER.len()` is fine).
pub struct Spinner {
    idx: usize,
}

impl Spinner {
    pub fn new(idx: usize) -> Self {
        Self { idx }
    }
}

impl Widget for Spinner {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let glyph = SPINNER[self.idx % SPINNER.len()];
        let span = Span::styled(glyph, theme::palette().spinner());
        span.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::terminal::Terminal;

    #[test]
    fn renders_spinner_glyph() {
        let mut term = Terminal::new(TestBackend::new(10, 1)).unwrap();
        term.draw(|f| Spinner::new(0).render(Rect::new(0, 0, 10, 1), f.buffer_mut()))
            .unwrap();
        let buf = term.backend().buffer();
        let glyph = buf[(0, 0)].symbol().chars().next().unwrap();
        assert!(SPINNER.contains(&glyph.to_string().as_str()));
    }
}
