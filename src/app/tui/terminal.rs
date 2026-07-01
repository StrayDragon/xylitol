//! Terminal lifecycle — RAII wrapper around the inline ratatui terminal.
//!
//! Enters raw mode + `Viewport::Inline` on construction (NOT the alternate
//! screen) and restores the terminal on [`Drop`], so the user's terminal is
//! never left in a broken raw-mode state — even on panic (spec tui15).

use std::io;

use ratatui::TerminalOptions;
use ratatui::Viewport;
use ratatui::text::Line;

use crate::app::tui::app::TuiApp;
use crate::app::tui::render;

/// Height (in rows) of the mutable tail region reserved by `Viewport::Inline`.
/// Layout (pi-style, bottom-aligned within this height):
///   [streaming assistant text]   ← only while a turn streams
///   [thinking/loading indicator] ← only while a turn streams (italic, dimmed)
///   [❯ input prompt]             ← always; background block
const TAIL_HEIGHT: u16 = 4;

/// Owns the inline terminal. Dropping restores raw mode + leaves scrollback.
pub struct InlineTerminal {
    term: ratatui::DefaultTerminal,
}

impl InlineTerminal {
    /// Enable raw mode and create an inline viewport (no alt screen).
    pub fn enter() -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let term = ratatui::try_init_with_options(TerminalOptions {
            viewport: Viewport::Inline(TAIL_HEIGHT),
        })?;
        Ok(Self { term })
    }

    /// Redraw the mutable tail region from the app state.
    pub fn draw_tail(&mut self, app: &TuiApp) -> io::Result<()> {
        self.term
            .draw(|frame| render::draw_tail_frame(frame, app))?;
        Ok(())
    }

    /// Commit finalized lines into the scrollback (never touched again).
    pub fn commit_to_scrollback(&mut self, lines: &[Line]) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }
        let height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
        self.term.insert_before(height, |buf| {
            let area = buf.area;
            for (i, line) in lines.iter().enumerate() {
                let y = i as u16;
                if y >= area.height {
                    break;
                }
                let row_area = ratatui::layout::Rect {
                    x: area.x,
                    y: area.y + y,
                    width: area.width,
                    height: 1,
                };
                line.clone().render(row_area, buf);
            }
        })?;
        Ok(())
    }
}

impl Drop for InlineTerminal {
    fn drop(&mut self) {
        // Order matters: restore ratatui state, then raw mode. Best-effort —
        // never panic in Drop.
        ratatui::restore();
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

// Re-export the ratatui Line rendering helper bound for use in commit. ratatui
// 0.30.2 exposes `Line::render` via the Widget API; we rely on it above. The
// import below keeps the trait in scope without an `extern`-style path.
use ratatui::widgets::Widget;

// `Widget::render` is called as `line.clone().render(area, buf)` in
// `commit_to_scrollback`; the `use` above brings the trait into scope.
