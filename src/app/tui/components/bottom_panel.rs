//! `BottomPanel` widget — the bordered, background-filled chrome container at
//! the bottom of the tail (c365 route B). It frames the input prompt so the
//! inline viewport's reserved area reads as a deliberate "input panel" rather
//! than empty terminal rows.
//!
//! Layout inside the panel:
//! ```text
//! ╭──────────────────────╱  ← top border
//! │ <input>                 ← InputPrompt (no ❯ prefix)
//! ╰──────────────────────╯  ← bottom border
//! ```
//! The panel fills whatever area it is given. When idle (no mutable line
//! above), `Tail` hands it the whole tail area → the remaining inner rows
//! carry the panel bg (panel interior, not empty terminal). When streaming,
//! the mutable line (thinking/reply) sits above the panel.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::Borders;

use crate::app::tui::app::TuiApp;
use crate::app::tui::components::input_prompt::InputPrompt;
use crate::app::tui::theme;

/// Renders the bottom chrome panel: a bordered, bg-filled block containing the
/// input prompt (bottom-anchored). Owns no state; reads the app each frame.
pub struct BottomPanel<'a> {
    app: &'a TuiApp,
}

impl<'a> BottomPanel<'a> {
    pub fn new(app: &'a TuiApp) -> Self {
        Self { app }
    }

    /// Inner rows the chrome needs: just the input (1 row, MVP single-line).
    pub fn inner_rows() -> u16 {
        1
    }

    /// Total panel height (inner rows + top/bottom borders).
    pub fn height() -> u16 {
        Self::inner_rows() + 2
    }
}

impl Widget for BottomPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let p = theme::palette();
        let block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .style(Style::default().bg(p.panel_bg()))
            .border_style(p.panel_border());
        // `Block::render` takes `self` (moves), so compute the inner rect first.
        let inner = block.inner(area);
        block.render(area, buf);

        // Input prompt at the bottom of the inner area.
        let input_y = inner.bottom().saturating_sub(1);
        InputPrompt::new(self.app.input_buffer()).render(
            Rect {
                x: inner.x,
                y: input_y,
                width: inner.width,
                height: 1,
            },
            buf,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tui::app::TuiApp;
    use ratatui_core::backend::TestBackend;
    use ratatui_core::terminal::Terminal;

    fn render(app: &TuiApp, width: u16, height: u16) -> ratatui_core::buffer::Buffer {
        let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
        term.draw(|f| {
            BottomPanel::new(app).render(Rect::new(0, 0, width, height), f.buffer_mut());
        })
        .unwrap();
        term.backend().buffer().clone()
    }

    fn row_text(buf: &ratatui_core::buffer::Buffer, y: u16, width: u16) -> String {
        (0..width)
            .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn panel_shows_borders_and_input_no_prefix() {
        let app = TuiApp::default();
        let buf = render(&app, 30, 5);
        let top = row_text(&buf, 0, 30);
        let bottom = row_text(&buf, 4, 30);
        assert!(top.starts_with('─'), "top border: {top:?}");
        assert!(bottom.starts_with('─'), "bottom border: {bottom:?}");
        // Input on the inner bottom row (row 3), no ❯ prefix.
        let input = row_text(&buf, 3, 30);
        assert!(
            input.is_empty() || !input.contains('❯'),
            "no prefix: {input:?}"
        );
    }

    #[test]
    fn panel_shows_typed_input() {
        let mut app = TuiApp::default();
        for c in "hello".chars() {
            app.push_char(c);
        }
        let buf = render(&app, 30, 5);
        assert_eq!(row_text(&buf, 3, 30), "hello");
    }

    #[test]
    fn panel_inner_filled_with_panel_bg() {
        let app = TuiApp::default();
        let buf = render(&app, 30, 5);
        let bg = theme::palette().panel_bg();
        for y in 1..4u16 {
            assert_eq!(buf[(0, y)].bg, bg, "inner row {y} carries panel_bg");
        }
    }

    #[test]
    fn height_is_input_plus_two_borders() {
        assert_eq!(BottomPanel::height(), 3);
    }
}
