//! `Tail` widget — composes [`MutableLine`] / [`ThinkingIndicator`] /
//! [`InputPrompt`] into the inline tail region, **bottom-anchored**.
//!
//! Layout while streaming (top → bottom):
//! ```text
//! [MutableLine wrapped rows]   ← transparent bg, blends into scrollback
//! [ThinkingIndicator]          ← input_bg
//! [InputPrompt]                ← bottom row, input_bg
//! ```
//! When the mutable wrapped rows + indicator + input exceed the area height,
//! the TOP mutable rows are dropped (oldest wrap results; the full text
//! commits to scrollback on the next newline). Bottom-anchored so future rows
//! (a status bar) can be pushed on from the bottom without rewriting the
//! layout (c365 future-layout extension point).
//!
//! `draw_tail_frame` delegates here; the frame cursor is positioned
//! separately via [`input_cursor_position`] (a `Widget` cannot set the frame
//! cursor).

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::Widget;

use crate::app::tui::app::TuiApp;
use crate::app::tui::components::input_prompt::{InputPrompt, cursor_x};
use crate::app::tui::components::mutable_line::MutableLine;
use crate::app::tui::components::thinking_indicator::ThinkingIndicator;

/// Renders the whole tail region from app state. Owns no state; reads the app
/// each frame (input buffer, spinner idx, pending tail, streaming flag).
pub struct Tail<'a> {
    app: &'a TuiApp,
    width: u16,
}

impl<'a> Tail<'a> {
    pub fn new(app: &'a TuiApp, width: u16) -> Self {
        Self { app, width }
    }
}

impl Widget for Tail<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the whole tail area first so no stale mutable text lingers
        // across frames (e.g. after TurnEnd the previous pending tail must
        // vanish). Each child repaints its own rows; uncleared rows stay
        // transparent (default bg) so the mutable line blends into scrollback.
        for y in area.top()..area.bottom() {
            for x in area.x..area.right() {
                buf[(x, y)].reset();
            }
        }
        // Build bottom-up. The input line is always pinned to the last row.
        let input_y = area.bottom().saturating_sub(1);
        InputPrompt::new(self.app.input_buffer()).render(
            Rect {
                x: area.x,
                y: input_y,
                width: area.width,
                height: 1,
            },
            buf,
        );

        // Thinking indicator sits directly above the input, only while streaming.
        let above_input = input_y;
        if self.app.is_streaming() {
            let indicator_y = above_input.saturating_sub(1);
            ThinkingIndicator::new(self.app.spinner_idx(), self.app.status_line()).render(
                Rect {
                    x: area.x,
                    y: indicator_y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
            // Mutable line fills the remaining rows above the indicator,
            // bottom-anchored within that sub-area (top rows dropped on overflow).
            if let Some(tail) = self.app.pending_tail()
                && indicator_y > area.y
            {
                let mutable_area = Rect {
                    x: area.x,
                    y: area.y,
                    width: area.width,
                    height: indicator_y - area.y,
                };
                MutableLine::new(tail, self.width).render(mutable_area, buf);
            }
        }
    }
}

/// Absolute cursor position for the input prompt within `area`, for the frame
/// to call `frame.set_cursor_position`. Always on the input (bottom) row.
pub fn input_cursor_position(area: Rect, app: &TuiApp) -> (u16, u16) {
    let y = area.bottom().saturating_sub(1);
    let x = area.x + cursor_x(app.input_buffer());
    (x.min(area.right().saturating_sub(1)), y)
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
            Tail::new(app, width).render(Rect::new(0, 0, width, height), f.buffer_mut());
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
    fn idle_tail_shows_only_input_at_bottom() {
        let app = TuiApp::default();
        let buf = render(&app, 40, 8);
        let last = row_text(&buf, 7, 40);
        assert!(last.starts_with('❯'), "input at bottom: {last}");
        // No spinner anywhere when idle.
        for y in 0..8u16 {
            assert!(!row_text(&buf, y, 40).contains('⠋'), "row {y} has spinner");
        }
    }

    #[test]
    fn streaming_shows_mutable_at_top_thinking_input_at_bottom() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("typing".into()));
        let buf = render(&app, 40, 8);
        let input = row_text(&buf, 7, 40);
        let indicator = row_text(&buf, 6, 40);
        let mutable = row_text(&buf, 0, 40);
        assert!(input.starts_with('❯'), "input row 7: {input}");
        assert!(indicator.starts_with('⠋'), "thinking row 6: {indicator}");
        // Top-anchored: the mutable line sits at row 0, flush against the
        // scrollback above (rows 1-5 stay empty — room to grow).
        assert_eq!(mutable, "typing", "mutable row 0: {mutable}");
    }

    #[test]
    fn mutable_overflows_drops_top_rows() {
        // 20 chars at width 4 = 5 mutable rows; with 1 indicator + 1 input
        // the mutable area has only 6 rows, so all 5 fit. Shrink to height 5:
        // mutable area = 3 rows → top 2 of 5 dropped, bottom 3 kept.
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("abcdefghijklmnopqrst".into()));
        let buf = render(&app, 4, 5);
        // rows 0..2 = mutable (bottom 3 of 5 -> "mnop","qrst"? width 4: rows
        // are "abcd","efgh","ijkl","mnop","qrst"; bottom 3 = "ijkl","mnop","qrst")
        assert_eq!(row_text(&buf, 0, 4), "ijkl");
        assert_eq!(row_text(&buf, 1, 4), "mnop");
        assert_eq!(row_text(&buf, 2, 4), "qrst");
        assert_eq!(row_text(&buf, 3, 4).chars().next(), Some('⠋'), "indicator");
        assert!(row_text(&buf, 4, 4).starts_with('❯'), "input");
    }

    #[test]
    fn mutable_row_is_transparent_indicator_and_input_have_bg() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("typing".into()));
        let buf = render(&app, 40, 8);
        let bg = crate::app::tui::theme::palette().input_bg();
        // input row 7 + indicator row 6 carry input_bg; mutable row 0 is Reset
        // (transparent, blends into the scrollback above).
        assert_eq!(buf[(0, 7)].bg, bg, "input bg");
        assert_eq!(buf[(0, 6)].bg, bg, "indicator bg");
        assert_eq!(
            buf[(0, 0)].bg,
            ratatui_core::style::Color::Reset,
            "mutable transparent"
        );
    }

    #[test]
    fn after_turn_end_no_reply_lingers_in_tail() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        app.handle_xy_event(XyEvent::TurnEnd { turn_index: 0 });
        app.end_stream();
        let buf = render(&app, 40, 8);
        // Only the input prompt remains; no "partial" anywhere in the tail.
        for y in 0..8u16 {
            let row = row_text(&buf, y, 40);
            assert!(!row.contains("partial"), "row {y} lingers: {row}");
        }
        assert!(row_text(&buf, 7, 40).starts_with('❯'));
    }
}
