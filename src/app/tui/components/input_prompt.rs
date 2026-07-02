//! `InputPrompt` widget — the input area (no `❯` prefix), rendered inside the
//! bottom panel. The cursor column is computed by [`cursor_x`] and applied by
//! the frame (`draw_tail_frame`), since a `Widget` cannot set the frame cursor
//! itself.
//!
//! MVP: single-line. A multi-line editor (up to 3 rows, arrow-key cursor) is a
//! planned follow-up — the widget API will grow a height/lines parameter then.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;

/// Renders the input line: the current input buffer text on a row filled with
/// the panel background. Always active (the user can type at any time, even
/// mid-stream — Enter interrupts + sends a new prompt). No `❯` prefix — the
/// bordered panel frame is the visual affordance (c365).
pub struct InputPrompt<'a> {
    input: &'a str,
}

impl<'a> InputPrompt<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }
}

impl Widget for InputPrompt<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // The panel block already fills the row bg; just paint the text.
        Line::raw(self.input).render(area, buf);
    }
}

/// Column offset of the cursor within an input prompt row: the display width
/// of the input buffer (CJK-aware). The caller adds the row's x origin to get
/// the absolute column for `frame.set_cursor_position`. No `❯` prefix offset
/// (c365: the prefix was removed).
pub fn cursor_x(input: &str) -> u16 {
    use unicode_width::UnicodeWidthStr;
    UnicodeWidthStr::width(input) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::terminal::Terminal;

    fn render(input: &str) -> ratatui_core::buffer::Buffer {
        let mut term = Terminal::new(TestBackend::new(40, 1)).unwrap();
        term.draw(|f| {
            InputPrompt::new(input).render(Rect::new(0, 0, 40, 1), f.buffer_mut());
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
    fn empty_input_shows_blank() {
        let buf = render("");
        assert_eq!(row_text(&buf), "");
    }

    #[test]
    fn shows_input_text() {
        let buf = render("hello");
        assert_eq!(row_text(&buf), "hello");
    }

    #[test]
    fn no_prompt_prefix() {
        let buf = render("hello");
        assert!(!row_text(&buf).contains('❯'), "no ❯ prefix: {buf:?}");
    }

    #[test]
    fn cursor_x_empty_is_zero() {
        assert_eq!(cursor_x(""), 0);
    }

    #[test]
    fn cursor_x_ascii_after_input() {
        assert_eq!(cursor_x("abc"), 3);
    }

    #[test]
    fn cursor_x_cjk_uses_display_width() {
        // "你好" = 4 display cols.
        assert_eq!(cursor_x("你好"), 4);
    }
}
