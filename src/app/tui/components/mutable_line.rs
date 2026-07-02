//! `MutableLine` widget — the un-terminated streaming tail rendered at the
//! top of the tail region, wrapped to width, with a transparent (default)
//! background so it blends into the scrollback above — visually the text
//! "grows in the body" rather than in a separate typing zone (c365 buffer route).
//!
//! When the wrapped rows exceed the given area height, the TOP (oldest) rows
//! are dropped: the bottom-anchored rows stay glued to the indicator below,
//! and the full text is never lost (it commits to scrollback on the next
//! newline boundary).

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::{Color, Style};
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;

use crate::app::tui::render::wrap_to_width;

/// Renders the pending (un-terminated) streaming text, wrapped to `width`,
/// transparent background, top-anchored within its area (top rows dropped on
/// overflow). The style is caller-supplied so the same widget renders both
/// thinking content (gray) and main reply text (normal).
pub struct MutableLine<'a> {
    text: &'a str,
    width: u16,
    style: Style,
}

impl<'a> MutableLine<'a> {
    pub fn new(text: &'a str, width: u16, style: Style) -> Self {
        Self { text, width, style }
    }

    /// Wrapped physical rows at `width`, styled with the caller-supplied style.
    pub fn rows(&self) -> Vec<Line<'static>> {
        wrap_to_width(self.text, self.width)
            .into_iter()
            .map(|row| Line::styled(row, self.style))
            .collect()
    }
}

impl Widget for MutableLine<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = self.rows();
        let total = rows.len();
        let capacity = area.height as usize;
        let fit = total.min(capacity);
        if fit == 0 {
            return;
        }
        // Top-anchored: the mutable line grows from the top of its area
        // (area.y), so it sits flush against the scrollback above. On overflow
        // (wrapped rows exceed capacity) the TOP rows are dropped: the bottom
        // rows (carrying the newest characters being typed) stay, starting at
        // area.y — so the line keeps its visual anchor at the scrollback while
        // the newest text remains visible. The full text commits to scrollback
        // on the next newline, so nothing is lost.
        let start = total.saturating_sub(fit);
        let top_y = area.y;
        for (i, row) in rows[start..].iter().enumerate() {
            let y = top_y + i as u16;
            if y >= area.bottom() {
                break;
            }
            let row_area = Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            };
            // Force a transparent background across the whole row so the
            // mutable line blends into the scrollback above (no input_bg block).
            for x in row_area.x..row_area.right() {
                buf[(x, y)].set_bg(Color::Reset);
            }
            row.render(row_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::buffer::CellWidth;
    use ratatui_core::terminal::Terminal;

    fn render(text: &str, width: u16, area_h: u16) -> ratatui_core::buffer::Buffer {
        let mut term = Terminal::new(TestBackend::new(width, area_h)).unwrap();
        term.draw(|f| {
            MutableLine::new(text, width, Style::default())
                .render(Rect::new(0, 0, width, area_h), f.buffer_mut());
        })
        .unwrap();
        term.backend().buffer().clone()
    }

    fn row_text(buf: &ratatui_core::buffer::Buffer, y: u16, width: u16) -> String {
        let mut out = String::new();
        let mut prev_w: u16 = 1;
        for x in 0..width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            let is_filler = sym.is_empty() || (sym == " " && prev_w == 2) || cell.cell_width() == 0;
            if is_filler {
                continue;
            }
            if let Some(ch) = sym.chars().next() {
                out.push(ch);
            }
            prev_w = cell.cell_width().max(1) as u16;
        }
        out.trim_end().to_string()
    }

    #[test]
    fn short_text_one_row_at_top_of_area() {
        let buf = render("hi", 40, 4);
        // Top-anchored: a 1-row text sits at the TOP (row 0), flush against the
        // scrollback above. Rows below stay empty (room to grow).
        assert_eq!(row_text(&buf, 0, 40), "hi");
        assert_eq!(row_text(&buf, 3, 40), "");
    }

    #[test]
    fn long_text_wraps_multiple_rows() {
        let buf = render("abcdefghij", 4, 4);
        // Top-anchored: wrap rows fill from row 0 downward.
        assert_eq!(row_text(&buf, 0, 4), "abcd");
        assert_eq!(row_text(&buf, 1, 4), "efgh");
        assert_eq!(row_text(&buf, 2, 4), "ij");
        assert_eq!(row_text(&buf, 3, 4), "");
    }

    #[test]
    fn overflow_drops_top_rows() {
        // 10 chars at width 4 = 3 rows, but area height 2 → only 2 rows fit.
        // Top rows dropped: the bottom 2 rows ("efgh", "ij" — newest text)
        // stay, drawn from row 0 so the line keeps its scrollback anchor.
        let buf = render("abcdefghij", 4, 2);
        assert_eq!(row_text(&buf, 0, 4), "efgh");
        assert_eq!(row_text(&buf, 1, 4), "ij");
    }

    #[test]
    fn transparent_background_blends_with_scrollback() {
        let buf = render("hi", 10, 1);
        // The mutable row carries Reset bg (transparent), NOT the input_bg block.
        assert_eq!(buf[(0, 0)].bg, Color::Reset);
    }
}
