//! Terminal lifecycle — RAII wrapper around the inline ratatui terminal.
//!
//! Enters raw mode + `Viewport::Inline` on construction (NOT the alternate
//! screen) and restores the terminal on [`Drop`], so the user's terminal is
//! never left in a broken raw-mode state — even on panic (spec tui15).

use std::io;

use ratatui_core::terminal::{TerminalOptions, Viewport};
use ratatui_core::text::Line;

use crate::app::tui::app::TuiApp;
use crate::app::tui::init;
use crate::app::tui::render;

/// Height (in rows) of the mutable tail region reserved by `Viewport::Inline`.
/// Layout (pi-style, bottom-aligned within this height):
///   [streaming assistant text]   ← only while a turn streams
///   [thinking/loading indicator] ← only while a turn streams (italic, dimmed)
///   [❯ input prompt]             ← always; background block
const TAIL_HEIGHT: u16 = 3;

/// Owns the inline terminal. Dropping restores raw mode + leaves scrollback.
pub struct InlineTerminal {
    term: init::DefaultTerminal,
}

impl InlineTerminal {
    /// Enable raw mode and create an inline viewport (no alt screen).
    pub fn enter() -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let term = init::try_init_with_options(TerminalOptions {
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
    ///
    /// Renders each line cell by cell (instead of `Line::render`) to prevent
    /// ratatui's CJK filler cells from appearing as visible spaces — when a
    /// CJK character occupies 2 terminal columns, the second cell has an empty
    /// symbol that `cell.symbol()` returns as `" "`, producing a visible space.
    pub fn commit_to_scrollback(&mut self, lines: &[Line]) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }
        let height = u16::try_from(lines.len()).unwrap_or(u16::MAX);
        self.term.insert_before(height, |buf| {
            let area = buf.area;
            for (i, line) in lines.iter().enumerate() {
                let y = area.y + i as u16;
                if y >= area.bottom() {
                    break;
                }
                // Reset row, then render spans cell by cell.
                for x in area.x..area.right() {
                    buf[(x, y)].reset();
                }
                let mut x = area.x;
                for span in &line.spans {
                    for ch in span.content.chars() {
                        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                        if w == 0 || w > 2 {
                            continue;
                        }
                        if x >= area.right() {
                            break;
                        }
                        buf[(x, y)].set_char(ch);
                        buf[(x, y)].set_style(span.style);
                        x += 1;
                        // For double-width (CJK), second column gets
                        // empty symbol so terminal outputs nothing.
                        if w == 2 && x < area.right() {
                            buf[(x, y)].set_symbol("");
                            buf[(x, y)].set_style(span.style);
                            x += 1;
                        }
                    }
                }
            }
        })?;
        Ok(())
    }
}

impl Drop for InlineTerminal {
    fn drop(&mut self) {
        // Order matters: restore ratatui state, then raw mode. Best-effort —
        // never panic in Drop. (init::restore already disables raw mode; the
        // explicit call below is a belt-and-suspenders redundancy.)
        init::restore();
        let _ = crossterm::terminal::disable_raw_mode();
    }
}
