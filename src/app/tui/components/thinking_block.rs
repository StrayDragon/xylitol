//! `ThinkingBlock` widget — the reasoning/thinking display, an INDEPENDENT
//! block separate from the working spinner (c365). Today it shows a `Thinking…`
//! label while a turn streams; later it will carry expandable reasoning content
//! (accumulated from `ThinkingDelta`), hot-toggleable between a collapsed label
//! and an expanded text view.
//!
//! Split from `StatusIndicator` deliberately: the spinner tracks EXECUTION
//! progress (Working / running tool X), the thinking block tracks REASONING
//! content. They evolve independently — e.g. a tool can run while reasoning is
//! collapsed or expanded.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;
use ratatui_core::widgets::Widget;

use crate::app::tui::theme;

/// Renders the thinking/reasoning block. While streaming and no reasoning
/// content has arrived, shows a `Thinking…` label (italic, dim). When
/// reasoning text is present, shows it (future: collapsed/expanded toggle).
pub struct ThinkingBlock<'a> {
    /// Accumulated reasoning text (from `ThinkingDelta`), if any. `None` →
    /// show the placeholder label.
    reasoning: Option<&'a str>,
}

impl<'a> ThinkingBlock<'a> {
    pub fn new(reasoning: Option<&'a str>) -> Self {
        Self { reasoning }
    }
}

impl Widget for ThinkingBlock<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = theme::palette();
        let text = self.reasoning.unwrap_or("Thinking…");
        // Single-line for now; future expansion will wrap reasoning into the
        // block's area and add a collapsed/expanded affordance.
        Line::styled(text.to_string(), p.thinking()).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::terminal::Terminal;

    fn render(reasoning: Option<&str>) -> ratatui_core::buffer::Buffer {
        let mut term = Terminal::new(TestBackend::new(40, 1)).unwrap();
        term.draw(|f| {
            ThinkingBlock::new(reasoning).render(Rect::new(0, 0, 40, 1), f.buffer_mut());
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
    fn no_reasoning_shows_thinking_placeholder() {
        let buf = render(None);
        assert_eq!(row_text(&buf), "Thinking…");
    }

    #[test]
    fn reasoning_text_shown_when_present() {
        let buf = render(Some("I should read the file first."));
        assert_eq!(row_text(&buf), "I should read the file first.");
    }
}
