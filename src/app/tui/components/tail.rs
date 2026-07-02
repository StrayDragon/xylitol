//! `Tail` widget — composes [`MutableLine`] (top, transparent) and
//! [`BottomPanel`] (bottom, bordered + bg-filled) into the inline tail region.
//!
//! Layout while streaming (top → bottom):
//! ```text
//! [MutableLine wrapped rows]   ← transparent bg, blends into scrollback
//! ╭──────────────────────╮     ← panel top border
//! │ ⠧ Working            │     ← StatusIndicator
//! │ Thinking…            │     ← ThinkingBlock (independent, future-expandable)
//! │ ❯ input              │     ← InputPrompt (bottom-anchored)
//! ╰──────────────────────╯     ← panel bottom border
//! ```
//! When idle, the mutable line is gone and the panel fills the WHOLE tail area
//! (its inner rows carry the panel bg) — so there are no empty terminal rows
//! above the input; the reserved viewport reads as a deliberate input panel
//! (c365 route B, fixes the post-turn empty-row artifact).
//!
//! `draw_tail_frame` delegates here; the frame cursor is positioned separately
//! via [`input_cursor_position`] (a `Widget` cannot set the frame cursor).

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::Widget;

use crate::app::tui::app::TuiApp;
use crate::app::tui::components::bottom_panel::BottomPanel;
use crate::app::tui::components::input_prompt::cursor_x;
use crate::app::tui::components::mutable_line::MutableLine;

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
        let streaming = self.app.is_streaming();
        let pending = self.app.pending_tail();
        let has_mutable = streaming && pending.is_some();

        // Panel height: while streaming it needs its chrome rows + borders;
        // when idle (no mutable) it fills the WHOLE area so no empty terminal
        // rows sit above the input (c365 route B).
        let panel_p = if has_mutable {
            BottomPanel::height(true)
        } else {
            area.height
        };
        let available_above = area.height.saturating_sub(panel_p);

        // Mutable line: sized to exactly the rows it wraps (capped to the
        // available space above the panel), placed flush against the panel top
        // (bottom of the available-above region). Top-anchored within that
        // small area → the text sits right above the panel border, growing
        // downward, transparent so it blends into the scrollback. Overflowing
        // wrap rows drop from the top (newest text stays visible).
        if let Some(tail) = pending
            && has_mutable
            && available_above > 0
        {
            let rows = MutableLine::new(tail, self.width).rows().len() as u16;
            let mutable_h = rows.min(available_above);
            let mutable_y = area.y + available_above - mutable_h;
            MutableLine::new(tail, self.width).render(
                Rect {
                    x: area.x,
                    y: mutable_y,
                    width: area.width,
                    height: mutable_h,
                },
                buf,
            );
        }

        // Bottom panel fills the rest (its area starts right below the
        // available-above region).
        let panel_area = Rect {
            x: area.x,
            y: area.y + available_above,
            width: area.width,
            height: panel_p,
        };
        BottomPanel::new(self.app).render(panel_area, buf);
    }
}

/// Absolute cursor position for the input prompt within `area`, for the frame
/// to call `frame.set_cursor_position`. Always on the input (bottom inner) row
/// of the panel: panel bottom border is the last area row, the input sits one
/// row above it.
pub fn input_cursor_position(area: Rect, app: &TuiApp) -> (u16, u16) {
    let streaming = app.is_streaming();
    let has_mutable = streaming && app.pending_tail().is_some();
    let panel_h = if has_mutable {
        BottomPanel::height(true)
    } else {
        area.height
    };
    let available_above = area.height.saturating_sub(panel_h);
    let panel_area = Rect {
        x: area.x,
        y: area.y + available_above,
        width: area.width,
        height: panel_h,
    };
    // Input is the last inner row of the panel (one row above the bottom
    // border). With TOP|BOTTOM borders only, the panel inner x equals the
    // panel area x (no left inset), so the cursor sits at panel_area.x +
    // the `❯ ` prefix width + the input display width.
    let y = panel_area.bottom().saturating_sub(2);
    let x = panel_area.x + cursor_x(app.input_buffer());
    (x.min(panel_area.right().saturating_sub(1)), y)
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
    fn idle_panel_fills_whole_area_no_empty_rows_above() {
        let app = TuiApp::default();
        let buf = render(&app, 30, 6);
        let bg = crate::app::tui::theme::palette().panel_bg();
        // Top border at row 0, bottom border at row 5, input at row 4.
        assert!(row_text(&buf, 0, 30).starts_with('─'), "top border");
        assert!(row_text(&buf, 5, 30).starts_with('─'), "bottom border");
        assert!(row_text(&buf, 4, 30).starts_with('❯'), "input row 4");
        // All inner rows carry panel bg (no empty terminal rows).
        for y in 1..5u16 {
            assert_eq!(buf[(0, y)].bg, bg, "inner row {y} carries panel_bg");
        }
    }

    #[test]
    fn streaming_mutable_top_then_panel_with_status_thinking_input() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("typing".into()));
        let buf = render(&app, 30, 6);
        // mutable at row 0 (top, transparent), panel below it.
        assert_eq!(row_text(&buf, 0, 30), "typing", "mutable row 0");
        // Panel (height 5) starts at row 1: border@1, status@2, thinking@3,
        // input@4, border@5.
        assert!(
            row_text(&buf, 1, 30).starts_with('─'),
            "panel top border row 1"
        );
        assert!(row_text(&buf, 2, 30).starts_with('⠋'), "status row 2");
        assert_eq!(row_text(&buf, 3, 30), "Thinking…", "thinking row 3");
        assert!(row_text(&buf, 4, 30).starts_with('❯'), "input row 4");
        assert!(
            row_text(&buf, 5, 30).starts_with('─'),
            "panel bottom border row 5"
        );
    }

    #[test]
    fn mutable_row_transparent_panel_inner_has_bg() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("typing".into()));
        let buf = render(&app, 30, 6);
        let bg = crate::app::tui::theme::palette().panel_bg();
        // Mutable row 0 is transparent (blends into scrollback).
        assert_eq!(
            buf[(0, 0)].bg,
            ratatui_core::style::Color::Reset,
            "mutable transparent"
        );
        // Panel inner rows (status 2, thinking 3, input 4) carry panel bg.
        assert_eq!(buf[(0, 2)].bg, bg, "status row bg");
        assert_eq!(buf[(0, 3)].bg, bg, "thinking row bg");
        assert_eq!(buf[(0, 4)].bg, bg, "input row bg");
    }

    #[test]
    fn after_turn_end_panel_fills_area_no_reply_lingers() {
        let mut app = TuiApp::default();
        app.start_stream();
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        app.handle_xy_event(XyEvent::TurnEnd { turn_index: 0 });
        app.end_stream();
        let buf = render(&app, 30, 6);
        for y in 0..6u16 {
            assert!(
                !row_text(&buf, y, 30).contains("partial"),
                "row {y} lingers"
            );
        }
        // Idle panel fills the area; input present, no spinner.
        assert!(row_text(&buf, 4, 30).starts_with('❯'), "input row");
        for y in 0..6u16 {
            assert!(!row_text(&buf, y, 30).contains('⠋'), "row {y} spinner");
        }
    }
}
