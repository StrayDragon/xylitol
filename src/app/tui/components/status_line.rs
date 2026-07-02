//! `StatusLine` widget — a fixed one-row status bar between the mutable
//! streaming line and the bottom input panel (c380).
//!
//! Renders a spinner glyph (while streaming, animated by the Tick-driven
//! `spinner_idx`) followed by an activity label (`Working…` / `Running {tool}`
//! / `Ready`). Left-aligned. Data-driven: consumes [`StatusSegments`] (from
//! `TuiApp::status_segments`) and does NOT match business state itself.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Layout, Rect};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use crate::app::tui::app::StatusSegments;
use crate::app::tui::components::Spinner;
use crate::app::tui::theme;

/// Renders the always-present status row: spinner + activity label.
///
/// `segments` carries the label + streaming flag; `spinner_idx` advances the
/// spinner glyph (only rendered while `segments.streaming`).
pub struct StatusLine<'a> {
    segments: &'a StatusSegments,
    spinner_idx: usize,
}

impl<'a> StatusLine<'a> {
    pub fn new(segments: &'a StatusSegments, spinner_idx: usize) -> Self {
        Self {
            segments,
            spinner_idx,
        }
    }
}

impl Widget for StatusLine<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let dim = theme::palette().text_dim();

        // spinner (streaming only) + label, left-aligned on one row.
        if self.segments.streaming {
            let [glyph_area, label_area] =
                Layout::horizontal([Constraint::Length(1), Constraint::Min(0)]).areas(area);
            Spinner::new(self.spinner_idx).render(glyph_area, buf);
            Line::from(Span::styled(format!(" {}", self.segments.left_label), dim))
                .render(label_area, buf);
        } else {
            Line::from(Span::styled(self.segments.left_label.clone(), dim)).render(area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::terminal::Terminal;

    fn render(segments: &StatusSegments, spinner_idx: usize, width: u16) -> Buffer {
        let mut term = Terminal::new(TestBackend::new(width, 1)).unwrap();
        term.draw(|f| {
            StatusLine::new(segments, spinner_idx).render(Rect::new(0, 0, width, 1), f.buffer_mut())
        })
        .unwrap();
        term.backend().buffer().clone()
    }

    fn row_text(buf: &Buffer, width: u16) -> String {
        (0..width)
            .map(|x| buf[(x, 0)].symbol().chars().next().unwrap_or(' '))
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn segments(streaming: bool, label: &str) -> StatusSegments {
        StatusSegments {
            streaming,
            left_label: label.into(),
        }
    }

    #[test]
    fn idle_shows_ready() {
        let s = segments(false, "Ready");
        let buf = render(&s, 0, 20);
        assert_eq!(row_text(&buf, 20), "Ready", "idle shows Ready, no spinner");
    }

    #[test]
    fn streaming_shows_spinner_and_working() {
        let s = segments(true, "Working…");
        let buf = render(&s, 0, 20);
        let text = row_text(&buf, 20);
        assert!(
            text.starts_with('⠋'),
            "streaming starts with spinner glyph: {text}"
        );
        assert!(text.contains("Working…"), "streaming shows Working: {text}");
    }

    #[test]
    fn tool_running_shows_tool_name() {
        let s = segments(true, "Running bash");
        let buf = render(&s, 3, 20);
        assert!(
            row_text(&buf, 20).contains("Running bash"),
            "shows tool name"
        );
    }

    #[test]
    fn spinner_advances_with_index() {
        let s = segments(true, "Working…");
        let a = render(&s, 0, 20);
        let b = render(&s, 1, 20);
        assert_ne!(
            a[(0, 0)].symbol(),
            b[(0, 0)].symbol(),
            "spinner glyph changes with index"
        );
    }
}
