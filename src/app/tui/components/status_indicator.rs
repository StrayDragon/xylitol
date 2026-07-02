//! `StatusIndicator` widget — animated spinner + a short status label, one
//! row. This is the generic "working" indicator (NOT the thinking block): its
//! label defaults to `Working` and is overridden by a concrete tool status
//! while a tool runs (e.g. `running read_file`).
//!
//! Reasoning/thinking display is a separate concern (`ThinkingBlock`) so the
//! two can evolve independently — the spinner tracks execution progress,
//! the thinking block will carry expandable reasoning content later.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use crate::app::tui::app::SPINNER;
use crate::app::tui::render::StatusLine;
use crate::app::tui::theme;

/// Renders the working/status indicator: spinner glyph + label. The label is
/// the active tool status if any, else `Working`.
pub struct StatusIndicator<'a> {
    spinner_idx: usize,
    status: Option<&'a StatusLine>,
}

impl<'a> StatusIndicator<'a> {
    pub fn new(spinner_idx: usize, status: Option<&'a StatusLine>) -> Self {
        Self {
            spinner_idx,
            status,
        }
    }

    fn label(&self) -> &str {
        self.status.map(|s| s.label()).unwrap_or("Working")
    }
}

impl Widget for StatusIndicator<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = theme::palette();
        let spinner = SPINNER[self.spinner_idx % SPINNER.len()];
        let line = Line::from(vec![
            Span::styled(spinner.to_string(), p.spinner()),
            Span::styled(format!(" {}", self.label()), p.thinking()),
        ]);
        // No per-row bg here: the BottomPanel fills its inner area with the
        // panel background; the indicator just paints its text on top.
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
            StatusIndicator::new(idx, status).render(Rect::new(0, 0, 40, 1), f.buffer_mut());
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
    fn default_label_is_working() {
        let buf = render(0, None);
        let row = row_text(&buf);
        assert!(row.starts_with(SPINNER[0]), "spinner glyph: {row}");
        assert!(row.contains("Working"), "default label: {row}");
    }

    #[test]
    fn status_label_overrides_default() {
        let s = StatusLine::new("⚙", " running read_file");
        let buf = render(2, Some(&s));
        let row = row_text(&buf);
        assert!(row.starts_with(SPINNER[2]), "spinner glyph idx 2: {row}");
        assert!(row.contains("running read_file"), "status label: {row}");
    }
}
