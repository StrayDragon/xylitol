//! `StatusLine` widget — a fixed one-row, three-segment status bar between the
//! mutable streaming line and the bottom input panel (c380).
//!
//! Layout (left → right):
//! ```text
//! {spinner} Working…      Turn 2        gpt-4o
//! └── left (左对齐) ──┘ └ center (居中) ┘└ right (右对齐) ┘
//! ```
//! - **left**: spinner glyph (while streaming, fixed at segment start) + an
//!   activity label (Working… / Running {tool} / Ready).
//! - **center**: ReAct iteration count (Turn {n}) while streaming.
//! - **right**: current model name.
//!
//! The widget is data-driven: it consumes [`StatusSegments`] (produced by
//! `TuiApp::status_segments`) plus the spinner frame index, and renders by
//! alignment. It does NOT match business state itself — adding a future status
//! item (token count, elapsed time) means extending `StatusSegments`, not this
//! widget. The three-segment skeleton mirrors vim airline / VSCode status bar.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Constraint, Layout, Rect};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::Widget;

use crate::app::tui::app::StatusSegments;
use crate::app::tui::components::Spinner;
use crate::app::tui::theme;

/// Renders the always-present three-segment status row.
///
/// `segments` carries the label/center/right content; `spinner_idx` advances
/// the spinner glyph (only rendered while `segments.streaming`).
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
        let p = theme::palette();
        let dim = p.text_dim();

        // Three segments: left grows, center is fixed-ish, right grows. Using
        // proportional constraints lets left and right fill while center sits
        // in the middle. A 3-way Min/Percentage split keeps center centered.
        let [left, center, right] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(area);

        // ── left: spinner (streaming only) + activity label ──────────
        let mut left_spans: Vec<Span> = Vec::new();
        if self.segments.streaming {
            // Render the spinner glyph as the first span of the left segment.
            // Spinner::new gives the glyph; we draw it into a 1-width sub-area
            // so the rest of the label follows on the same row.
            let [glyph_area, label_area] =
                Layout::horizontal([Constraint::Length(1), Constraint::Min(0)]).areas(left);
            Spinner::new(self.spinner_idx).render(glyph_area, buf);
            left_spans.push(Span::raw(" "));
            left_spans.push(Span::styled(self.segments.left_label.clone(), dim));
            Line::from(left_spans).render(label_area, buf);
        } else {
            left_spans.push(Span::styled(self.segments.left_label.clone(), dim));
            Line::from(left_spans).render(left, buf);
        }

        // ── center: Turn {n} (streaming only) ────────────────────────
        if let Some(center_text) = &self.segments.center {
            Line::from(Span::styled(center_text.clone(), dim))
                .alignment(Alignment::Center)
                .render(center, buf);
        }

        // ── right: model name ────────────────────────────────────────
        if let Some(model) = &self.segments.right {
            Line::from(Span::styled(model.clone(), dim))
                .alignment(Alignment::Right)
                .render(right, buf);
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

    fn segments(
        streaming: bool,
        label: &str,
        center: Option<&str>,
        right: Option<&str>,
    ) -> StatusSegments {
        StatusSegments {
            streaming,
            left_label: label.into(),
            center: center.map(String::from),
            right: right.map(String::from),
        }
    }

    #[test]
    fn idle_shows_ready_and_model() {
        let s = segments(false, "Ready", None, Some("gpt-4o"));
        let buf = render(&s, 0, 30);
        let text = row_text(&buf, 30);
        assert!(text.contains("Ready"), "left shows Ready: {text}");
        assert!(text.contains("gpt-4o"), "right shows model: {text}");
    }

    #[test]
    fn streaming_shows_spinner_working_turn_model() {
        let s = segments(true, "Working…", Some("Turn 2"), Some("gpt-4o"));
        let buf = render(&s, 0, 40);
        let text = row_text(&buf, 40);
        assert!(
            text.starts_with('⠋'),
            "left starts with spinner glyph: {text}"
        );
        assert!(text.contains("Working…"), "left has Working: {text}");
        assert!(text.contains("Turn 2"), "center has turn: {text}");
        assert!(text.ends_with("gpt-4o"), "right has model: {text}");
    }

    #[test]
    fn tool_running_shows_tool_name() {
        let s = segments(true, "Running bash", Some("Turn 1"), Some("gpt-4o"));
        let buf = render(&s, 3, 60);
        let text = row_text(&buf, 60);
        assert!(text.contains("Running bash"), "left shows tool: {text}");
    }

    #[test]
    fn spinner_advances_with_index() {
        let s = segments(true, "Working…", None, None);
        let a = render(&s, 0, 20);
        let b = render(&s, 1, 20);
        let ga = a[(0, 0)].symbol().to_string();
        let gb = b[(0, 0)].symbol().to_string();
        assert_ne!(ga, gb, "spinner glyph changes with index");
    }

    #[test]
    fn idle_has_no_spinner() {
        let s = segments(false, "Ready", None, None);
        let buf = render(&s, 0, 20);
        let first = buf[(0, 0)].symbol().to_string();
        assert_eq!(
            first, "R",
            "idle left starts with label 'R'eady, no spinner glyph"
        );
    }
}
