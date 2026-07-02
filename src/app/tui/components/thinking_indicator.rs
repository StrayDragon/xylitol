//! `ThinkingIndicator` widget — animated spinner glyph + status label, one
//! row filled with the input background block. Shown only while a turn streams.
//!
//! The spinner glyph cycle (`SPINNER`) is owned by the app state machine and
//! reused here so the tick-driven `spinner_idx` and the rendered glyph stay in sync.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use crate::app::tui::app::SPINNER;
use crate::app::tui::render::StatusLine;
use crate::app::tui::theme;

/// Renders the thinking/status indicator: spinner glyph + label, on a row
/// filled with the input background block.
pub struct ThinkingIndicator<'a> {
    spinner_idx: usize,
    status: Option<&'a StatusLine>,
}

impl<'a> ThinkingIndicator<'a> {
    pub fn new(spinner_idx: usize, status: Option<&'a StatusLine>) -> Self {
        Self {
            spinner_idx,
            status,
        }
    }

    fn label(&self) -> String {
        self.status
            .map(|s| s.label().to_string())
            .unwrap_or_else(|| "Thinking…".to_string())
    }
}

impl Widget for ThinkingIndicator<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = theme::palette();
        let spinner = SPINNER[self.spinner_idx % SPINNER.len()];
        let line = Line::from(vec![
            Span::styled(spinner.to_string(), p.spinner()),
            Span::styled(format!(" {}", self.label()), p.thinking()),
        ]);
        // Fill the whole row with the input background block (pi-style).
        let bg = p.input_bg();
        for x in area.x..area.right() {
            buf[(x, area.y)].set_bg(bg);
        }
        line.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::terminal::Terminal;

    fn render(idx: usize, status: Option<&StatusLine>) -> ratatui_core::buffer::Buffer {
        let mut term = Terminal::new(TestBackend::new(40, 1)).unwrap();
        term.draw(|f| {
            ThinkingIndicator::new(idx, status).render(Rect::new(0, 0, 40, 1), f.buffer_mut());
        })
        .unwrap();
        term.backend().buffer().clone()
    }

    fn row_text(buf: &ratatui_core::buffer::Buffer) -> String {
        (0..40u16)
            .map(|x| buf[(x, 0)].symbol().chars().next().unwrap_or(' '))
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn default_label_is_thinking() {
        let buf = render(0, None);
        let row = row_text(&buf);
        assert!(row.starts_with(SPINNER[0]), "spinner glyph: {row}");
        assert!(row.contains("Thinking…"), "default label: {row}");
    }

    #[test]
    fn status_label_overrides_default() {
        let s = StatusLine::new("⚙", " running read_file");
        let buf = render(2, Some(&s));
        let row = row_text(&buf);
        assert!(row.starts_with(SPINNER[2]), "spinner glyph idx 2: {row}");
        assert!(row.contains("running read_file"), "status label: {row}");
    }

    #[test]
    fn carries_input_background_block() {
        let buf = render(0, None);
        let expected_bg = theme::palette().input_bg();
        for x in 0..40u16 {
            assert_eq!(buf[(x, 0)].bg, expected_bg, "col {x} should carry input_bg");
        }
    }
}
