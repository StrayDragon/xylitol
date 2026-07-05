//! `Tail` widget — composes [`MutableLine`] (top, transparent), [`StatusLine`]
//! (middle, fixed 1 row), and [`BottomPanel`] (bottom, bordered + bg-filled)
//! into the inline tail region.
//!
//! Layout while streaming (top → bottom):
//! ```text
//! [MutableLine: thinking or reply text]  ← transparent, flush against scrollback
//! {spinner} Working…      Turn 2     gpt-4o  ← StatusLine (c380), always present
//! ────────────────                       ← panel top border
//! <input>                                ← InputPrompt (no ❯ prefix)
//! ────────────────                       ← panel bottom border
//! ```
//! The mutable line shows whichever phase is active:
//! - **Thinking phase** (before first TextDelta): `Thinking…` placeholder (gray)
//!   or streaming reasoning content (gray, from `ThinkingDelta`).
//! - **Text phase**: streaming reply text (normal color, from `TextDelta`).
//! - **Tool running** (no pending text): the tool status label (dim).
//!
//! When idle, the status line shows `Ready` + model name; the mutable line is
//! absent and the panel fills the bottom 3 rows.

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::Widget;

use crate::app::tui::app::TuiApp;
use crate::app::tui::components::StatusLine;
use crate::app::tui::components::bottom_panel::BottomPanel;
use crate::app::tui::components::input_prompt::cursor_x;
use crate::app::tui::components::mutable_line::MutableLine;

/// Renders the whole tail region from app state. Owns no state; reads the app
/// each frame (input buffer, pending tail + kind, streaming flag, status
/// segments, spinner index).
pub struct Tail<'a> {
    app: &'a TuiApp,
}

impl<'a> Tail<'a> {
    pub fn new(app: &'a TuiApp) -> Self {
        Self { app }
    }
}

impl Widget for Tail<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let streaming = self.app.is_streaming();
        let pending = self.app.pending_tail();
        let has_mutable = streaming && pending.is_some();

        // Fixed bottom stack: panel (3 rows) + status line (1 row) sit at the
        // bottom; the mutable line fills whatever remains above.
        let panel_h = BottomPanel::height();
        let status_h: u16 = 1;
        let available_for_mutable = area.height.saturating_sub(panel_h + status_h);

        // Mutable line: sized to its rendered rows (capped to available space),
        // placed flush above the status line. Top-anchored within that small
        // area → text sits right above the status line, growing downward.
        // c376: pending_tail returns pre-highlighted Vec<Line> (not plain text).
        if let Some(lines) = pending
            && has_mutable
            && available_for_mutable > 0
        {
            let rows = lines.len() as u16;
            let mutable_h = rows.min(available_for_mutable);
            let mutable_y = area.y + available_for_mutable - mutable_h;
            MutableLine::from_lines(&lines).render(
                Rect {
                    x: area.x,
                    y: mutable_y,
                    width: area.width,
                    height: mutable_h,
                },
                buf,
            );
        }

        // Status line: fixed 1 row, sitting directly above the panel.
        let status_y = area.y + area.height - panel_h - status_h;
        StatusLine::new(&self.app.status_segments(), self.app.spinner_idx()).render(
            Rect {
                x: area.x,
                y: status_y,
                width: area.width,
                height: status_h,
            },
            buf,
        );

        // Bottom panel fills the bottom `panel_h` rows.
        let panel_area = Rect {
            x: area.x,
            y: area.y + area.height - panel_h,
            width: area.width,
            height: panel_h,
        };
        BottomPanel::new(self.app).render(panel_area, buf);
    }
}

/// Absolute cursor position for the input prompt within `area`, for the frame
/// to call `frame.set_cursor_position`. Always on the input (bottom inner) row
/// of the panel. Panel is fixed 3 rows; cursor sits one row above the area
/// bottom (above the bottom border).
pub fn input_cursor_position(area: Rect, app: &TuiApp) -> (u16, u16) {
    // Panel is always at the bottom: bottom border = area.bottom()-1,
    // input = area.bottom()-2.
    let y = area.bottom().saturating_sub(2);
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
            Tail::new(app).render(Rect::new(0, 0, width, height), f.buffer_mut());
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
    fn idle_panel_fixed_3_rows_at_bottom() {
        let app = TuiApp::default();
        let buf = render(&app, 30, 5);
        // Panel always 3 rows at the bottom: border@2, input@3, border@4.
        // Rows 0-1 are transparent terminal background (body area).
        assert!(
            row_text(&buf, 2, 30).starts_with('─'),
            "panel top border row 2"
        );
        assert!(
            row_text(&buf, 4, 30).starts_with('─'),
            "panel bottom border row 4"
        );
        // Input row (3) has the cursor-ready position.
        // Rows 0-1 should be transparent (not panel bg).
        assert_eq!(
            buf[(0, 0)].bg,
            ratatui_core::style::Color::Reset,
            "row 0 transparent"
        );
        assert_eq!(
            buf[(0, 1)].bg,
            ratatui_core::style::Color::Reset,
            "row 1 transparent"
        );
    }

    #[test]
    fn streaming_thinking_phase_shows_placeholder_above_panel() {
        let mut app = TuiApp::default();
        app.start_stream(80);
        // No ThinkingDelta yet → placeholder "Thinking…" (gray).
        // height=5: mutable@0, status@1, panel border@2/input@3/border@4 (c380).
        let buf = render(&app, 30, 5);
        assert_eq!(row_text(&buf, 0, 30), "Thinking…", "placeholder row 0");
        // Status line at row 1 (always present while streaming, c380).
        assert!(
            row_text(&buf, 1, 30).contains("Working"),
            "status row 1: {:?}",
            row_text(&buf, 1, 30)
        );
        assert!(
            row_text(&buf, 2, 30).starts_with('─'),
            "panel top border row 2"
        );
        assert!(
            row_text(&buf, 4, 30).starts_with('─'),
            "panel bottom border row 4"
        );
    }

    #[test]
    fn streaming_thinking_content_shows_gray_above_panel() {
        let mut app = TuiApp::default();
        app.start_stream(80);
        app.handle_xy_event(XyEvent::ThinkingDelta("reasoning here".into()));
        let buf = render(&app, 30, 5);
        // mutable@0 (status line pushed it up by one, c380).
        assert_eq!(
            row_text(&buf, 0, 30),
            "reasoning here",
            "thinking content row 0"
        );
    }

    #[test]
    fn streaming_text_phase_shows_reply_above_panel() {
        let mut app = TuiApp::default();
        app.start_stream(80);
        // Transition to text phase: a TextDelta flushes thinking.
        app.handle_xy_event(XyEvent::TextDelta("reply text".into()));
        let buf = render(&app, 30, 5);
        assert_eq!(row_text(&buf, 0, 30), "reply text", "reply content row 0");
    }

    #[test]
    fn mutable_row_transparent_panel_inner_has_bg() {
        let mut app = TuiApp::default();
        app.start_stream(80);
        app.handle_xy_event(XyEvent::TextDelta("typing".into()));
        let buf = render(&app, 30, 5);
        let bg = crate::app::tui::theme::palette().panel_bg();
        // Mutable row 1 is transparent.
        assert_eq!(
            buf[(0, 1)].bg,
            ratatui_core::style::Color::Reset,
            "mutable transparent"
        );
        // Panel inner row 3 carries panel bg.
        assert_eq!(buf[(0, 3)].bg, bg, "input row bg");
    }

    #[test]
    fn after_turn_end_panel_fills_area_no_reply_lingers() {
        let mut app = TuiApp::default();
        app.start_stream(80);
        app.handle_xy_event(XyEvent::TextDelta("partial".into()));
        app.handle_xy_event(XyEvent::TurnEnd { turn_index: 0 });
        app.end_stream();
        let buf = render(&app, 30, 5);
        for y in 0..5u16 {
            assert!(
                !row_text(&buf, y, 30).contains("partial"),
                "row {y} lingers"
            );
        }
    }

    #[test]
    fn no_prompt_prefix_in_input() {
        let mut app = TuiApp::default();
        for c in "hello".chars() {
            app.push_char(c);
        }
        let buf = render(&app, 30, 5);
        // Input row (3) has the text but no ❯.
        let input = row_text(&buf, 3, 30);
        assert_eq!(input, "hello");
        assert!(!input.contains('❯'), "no prefix");
    }
}
