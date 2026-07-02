//! `InputPrompt` widget — the `❯` prompt + current input buffer + input
//! background block, bottom-anchored. The cursor column is computed by
//! [`cursor_x`] and applied by the frame (`draw_tail_frame`), since a `Widget`
//! cannot set the frame cursor itself.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;

use crate::app::tui::theme;

/// Renders the input prompt line: `❯ <input>` on a row filled with the input
/// background block. Always active (the user can type at any time, even
/// mid-stream — Enter interrupts + sends a new prompt).
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
        let p = theme::palette();
        let prompt = format!("❯ {}", self.input);
        let style = Style::default().bg(p.input_bg());
        let line = Line::styled(prompt, style);
        // Fill the whole row with the input background block.
        let bg = p.input_bg();
        for x in area.x..area.right() {
            buf[(x, area.y)].set_bg(bg);
        }
        line.render(area, buf);
    }
}

/// Column offset of the cursor within an input prompt row: the `❯ ` prefix
/// width plus the display width of the input buffer (CJK-aware). The caller
/// adds the row's x origin to get the absolute column for
/// `frame.set_cursor_position`.
pub fn cursor_x(input: &str) -> u16 {
    use unicode_width::UnicodeWidthStr;
    let prefix = UnicodeWidthStr::width("❯ ") as u16;
    let input_w = UnicodeWidthStr::width(input) as u16;
    prefix + input_w
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
    fn empty_input_shows_prefix() {
        let buf = render("");
        assert_eq!(row_text(&buf), "❯");
    }

    #[test]
    fn shows_prefix_and_input() {
        let buf = render("hello");
        assert_eq!(row_text(&buf), "❯ hello");
    }

    #[test]
    fn carries_input_background_block() {
        let buf = render("x");
        let expected_bg = theme::palette().input_bg();
        for x in 0..40u16 {
            assert_eq!(buf[(x, 0)].bg, expected_bg, "col {x} should carry input_bg");
        }
    }

    #[test]
    fn cursor_x_empty_is_prefix_width() {
        // "❯ " is 2 display cols.
        assert_eq!(cursor_x(""), 2);
    }

    #[test]
    fn cursor_x_ascii_after_input() {
        assert_eq!(cursor_x("abc"), 2 + 3);
    }

    #[test]
    fn cursor_x_cjk_uses_display_width() {
        // "你好" = 4 display cols; cursor sits at 2 + 4 = 6.
        assert_eq!(cursor_x("你好"), 6);
    }
}
