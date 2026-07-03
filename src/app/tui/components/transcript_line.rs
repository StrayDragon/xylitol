//! `TranscriptLine` widget — render one finalized [`RenderedLine`] into a
//! buffer area, CJK-aware wrapping via the ratatui-widgets `Paragraph`.
//!
//! This is the single rendering core for finalized scrollback lines: it is used
//! by the `insert_before` commit path (`InlineTerminal::commit_to_scrollback`)
//! and by `TestBackend` tests. Splitting it out as a widget (c365) lets the
//! commit path and the render harness share one verified implementation
//! instead of duplicating a cell loop. Since c355, `UserInput`/`AssistantText`
//! render through `MarkdownRenderer` (`to_lines`), so this widget may produce
//! multiple styled lines per `RenderedLine`.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;
use ratatui_widgets::paragraph::Paragraph;

use crate::app::tui::render::RenderedLine;

/// Renders a single finalized [`RenderedLine`] into a buffer area, wrapping at
/// `width` (CJK-aware). The caller MUST size the area height to
/// [`TranscriptLine::row_count`] so the whole line is visible.
pub struct TranscriptLine<'a> {
    rendered: &'a RenderedLine,
    width: u16,
}

impl<'a> TranscriptLine<'a> {
    pub fn new(rendered: &'a RenderedLine, width: u16) -> Self {
        Self { rendered, width }
    }

    /// The wrapped physical rows at `width`. Used by the commit path to size
    /// the `insert_before` area before rendering. Since c355, markdown variants
    /// (`UserInput`/`AssistantText`) yield multiple already-wrapped lines via
    /// `to_lines`; other variants return a single line (wrapped by `Paragraph`
    /// at render time).
    pub fn rows(&self) -> Vec<Line<'static>> {
        self.rendered.to_lines(self.width)
    }

    /// Number of physical rows this line occupies at `width`.
    pub fn row_count(&self) -> u16 {
        u16::try_from(self.rows().len()).unwrap_or(u16::MAX)
    }
}

impl Widget for TranscriptLine<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // `rows()` already wraps markdown variants to `width`; non-markdown
        // single-line variants get reflowed by `Paragraph` (CJK-correct).
        let text = ratatui_core::text::Text::from(self.rows());
        Paragraph::new(text).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    //! TestBackend harness for TranscriptLine: ASCII / CJK / long-wrap, each
    //! asserted via the shared row-text helper so the assertions are
    //! implementation-agnostic (spec tui41).

    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::buffer::{Buffer, CellWidth};
    use ratatui_core::terminal::Terminal;

    fn render_to_buf(rendered: &RenderedLine, width: u16) -> (Buffer, u16) {
        let widget = TranscriptLine::new(rendered, width);
        let height = widget.row_count().max(1);
        let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
        term.draw(|f| {
            let area = Rect::new(0, 0, width, height);
            TranscriptLine::new(rendered, width).render(area, f.buffer_mut());
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        (buf, height)
    }

    fn row_text(buf: &Buffer, y: u16, width: u16) -> String {
        let mut out = String::new();
        let mut prev_width: u16 = 1;
        for x in 0..width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            let is_filler =
                sym.is_empty() || (sym == " " && prev_width == 2) || cell.cell_width() == 0;
            if is_filler {
                continue;
            }
            if let Some(ch) = sym.chars().next() {
                out.push(ch);
            }
            prev_width = cell.cell_width().max(1) as u16;
        }
        out.trim_end().to_string()
    }

    #[test]
    fn ascii_short_line_one_row() {
        let (buf, h) = render_to_buf(&RenderedLine::AssistantText("hello".into()), 40);
        assert_eq!(h, 1);
        assert_eq!(row_text(&buf, 0, 40), "hello");
    }

    #[test]
    fn long_line_wraps_to_width() {
        let (buf, h) = render_to_buf(&RenderedLine::AssistantText("abcdefghij".into()), 4);
        assert_eq!(h, 3, "10 chars at width 4 -> 3 rows");
        assert_eq!(row_text(&buf, 0, 4), "abcd");
        assert_eq!(row_text(&buf, 1, 4), "efgh");
        assert_eq!(row_text(&buf, 2, 4), "ij");
    }

    #[test]
    fn cjk_line_double_width_visible() {
        let (buf, _) = render_to_buf(&RenderedLine::AssistantText("你好".into()), 40);
        assert_eq!(row_text(&buf, 0, 40), "你好");
    }

    #[test]
    fn cjk_long_line_wraps_by_display_width() {
        let (buf, h) = render_to_buf(&RenderedLine::AssistantText("你好世界再见".into()), 4);
        assert_eq!(h, 3, "6 CJK chars at width 4 -> 3 rows of 2 chars");
        assert_eq!(row_text(&buf, 0, 4), "你好");
        assert_eq!(row_text(&buf, 1, 4), "世界");
        assert_eq!(row_text(&buf, 2, 4), "再见");
    }

    #[test]
    fn user_input_renders_with_prompt_prefix() {
        let (buf, _) = render_to_buf(&RenderedLine::UserInput("fix it".into()), 40);
        let row = row_text(&buf, 0, 40);
        assert!(row.contains('❯'), "prefix present: {row}");
        assert!(row.contains("fix it"), "prompt text: {row}");
    }

    #[test]
    fn tool_summary_renders_name_and_preview() {
        let (buf, _) = render_to_buf(
            &RenderedLine::ToolSummary {
                name: "read_file".into(),
                preview: "ok".into(),
                is_error: false,
            },
            40,
        );
        let row = row_text(&buf, 0, 40);
        assert!(row.contains("read_file"), "tool name: {row}");
        assert!(row.contains("ok"), "preview: {row}");
    }
}
