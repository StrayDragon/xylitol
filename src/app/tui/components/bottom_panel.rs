//! `BottomPanel` widget — the bordered, background-filled chrome container at
//! the bottom of the tail (c365 route B). It frames the working spinner, the
//! thinking block, and the input prompt so the inline viewport's reserved
//! area reads as a deliberate "input panel" rather than empty terminal rows
//! (fixes the post-turn empty-row artifact: idle rows become panel interior).
//!
//! Layout inside the panel (top → bottom), only while streaming:
//! ```text
//! ╭──────────────────────╮   ← top border
//! │ ⠧ Working            │   ← StatusIndicator (spinner + label)
//! │ Thinking…            │   ← ThinkingBlock (independent, future-expandable)
//! │ ❯ input              │   ← InputPrompt (bottom-anchored)
//! ╰──────────────────────╯   ← bottom border
//! ```
//! When idle, only the input prompt is shown inside the panel; the remaining
//! inner rows carry the panel background (panel interior, not empty terminal).
//! The panel fills whatever area it is given, so `Tail` can hand it the whole
//! tail area when idle (no mutable line) → zero empty rows above.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::style::Style;
use ratatui_core::widgets::Widget;
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::Borders;

use crate::app::tui::app::TuiApp;
use crate::app::tui::components::input_prompt::InputPrompt;
use crate::app::tui::components::status_indicator::StatusIndicator;
use crate::app::tui::components::thinking_block::ThinkingBlock;
use crate::app::tui::theme;

/// Renders the bottom chrome panel: a bordered, bg-filled block containing the
/// status indicator + thinking block (while streaming) and the input prompt
/// (always, bottom-anchored). Owns no state; reads the app each frame.
pub struct BottomPanel<'a> {
    app: &'a TuiApp,
}

impl<'a> BottomPanel<'a> {
    pub fn new(app: &'a TuiApp) -> Self {
        Self { app }
    }

    /// Inner rows the chrome needs: status + thinking + input while streaming,
    /// just input when idle. Used by `Tail` to size the panel before rendering.
    pub fn inner_rows(streaming: bool) -> u16 {
        if streaming {
            3 // status + thinking + input
        } else {
            1 // input only
        }
    }

    /// Total panel height (inner rows + top/bottom borders).
    pub fn height(streaming: bool) -> u16 {
        Self::inner_rows(streaming) + 2
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
        let streaming = self.app.is_streaming();

        if streaming {
            // Status indicator at the top of the inner area.
            let status_y = inner.y;
            StatusIndicator::new(self.app.spinner_idx(), self.app.status_line()).render(
                Rect {
                    x: inner.x,
                    y: status_y,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
            // Thinking block directly below the status indicator.
            let thinking_y = status_y.saturating_add(1);
            if thinking_y < inner.bottom() {
                ThinkingBlock::new(self.app.reasoning()).render(
                    Rect {
                        x: inner.x,
                        y: thinking_y,
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );
            }
        }

        // Input prompt always at the bottom of the inner area.
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
    use crate::domain::lifecycle::XyEvent;
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
    fn idle_panel_shows_borders_and_input_only() {
        let app = TuiApp::default();
        let buf = render(&app, 30, 6);
        // Top + bottom border rows (plain horizontal lines, no side borders →
        // no corner glyphs). Input at the inner bottom row (row 4).
        let top = row_text(&buf, 0, 30);
        let bottom = row_text(&buf, 5, 30);
        assert!(top.starts_with('─'), "top border: {top:?}");
        assert!(bottom.starts_with('─'), "bottom border: {bottom:?}");
        assert!(row_text(&buf, 4, 30).starts_with('❯'), "input row 4");
        // No spinner / thinking when idle.
        for y in 0..6u16 {
            let r = row_text(&buf, y, 30);
            assert!(!r.contains('⠋'), "row {y} spinner");
            assert!(!r.contains("Working"), "row {y} working");
        }
    }

    #[test]
    fn streaming_panel_shows_status_then_thinking_then_input() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("typing".into()));
        let buf = render(&app, 30, 6);
        // inner rows 1..5: status(1), thinking(2), input(4). Borders at 0 and 5.
        assert!(row_text(&buf, 1, 30).starts_with('⠋'), "status row 1");
        assert!(row_text(&buf, 1, 30).contains("Working"), "status label");
        assert_eq!(row_text(&buf, 2, 30), "Thinking…", "thinking row 2");
        assert!(row_text(&buf, 4, 30).starts_with('❯'), "input row 4");
    }

    #[test]
    fn panel_inner_filled_with_panel_bg_when_idle() {
        let app = TuiApp::default();
        let buf = render(&app, 30, 6);
        let bg = theme::palette().panel_bg();
        // Inner rows (1..5) carry the panel bg, not Reset (empty terminal).
        for y in 1..5u16 {
            assert_eq!(buf[(0, y)].bg, bg, "inner row {y} should carry panel_bg");
        }
    }

    #[test]
    fn height_helpers() {
        assert_eq!(BottomPanel::height(false), 3, "idle: input + 2 borders");
        assert_eq!(
            BottomPanel::height(true),
            5,
            "streaming: 3 inner + 2 borders"
        );
    }
}
