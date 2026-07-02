//! Terminal lifecycle — RAII wrapper around the inline ratatui terminal.
//!
//! Enters raw mode + `Viewport::Inline` on construction (NOT the alternate
//! screen) and restores the terminal on [`Drop`], so the user's terminal is
//! never left in a broken raw-mode state — even on panic (spec tui15).
//!
//! Before entering, the cursor is pushed to the bottom of the terminal so the
//! inline viewport anchors to the bottom edge (otherwise `Viewport::Inline(N)`
//! reserves N rows at the current cursor position, which may be mid-terminal
//! on startup, leaving the input box "floating" above the bottom).

use std::io;

use ratatui_core::terminal::{TerminalOptions, Viewport};

use crate::app::tui::app::TuiApp;
use crate::app::tui::init;
use crate::app::tui::render::{self, RenderedLine};

/// Height (in rows) of the mutable tail region reserved by `Viewport::Inline`.
/// Layout (bottom-anchored, c365 buffer route, rendered by the `Tail` widget):
/// the mutable streaming line (transparent bg, blends into scrollback), the
/// thinking indicator, and the input prompt. Kept compact (4) so the inline
/// viewport's reserved area leaves few empty rows when idle; the mutable line
/// wraps up to 2 rows before overflowing (top rows dropped, full text commits
/// on the next newline).
const TAIL_HEIGHT: u16 = 4;

/// Owns the inline terminal. Dropping restores raw mode + leaves scrollback.
pub struct InlineTerminal {
    term: init::DefaultTerminal,
}

impl InlineTerminal {
    /// Enable raw mode and create an inline viewport (no alt screen).
    pub fn enter() -> io::Result<Self> {
        // Push the cursor to the bottom so the viewport anchors there.
        if let Ok((_, h)) = crossterm::terminal::size() {
            use std::io::Write;
            let mut stdout = std::io::stdout();
            for _ in 0..h {
                let _ = writeln!(stdout);
            }
            let _ = stdout.flush();
        }
        crossterm::terminal::enable_raw_mode()?;
        let term = init::try_init_with_options(TerminalOptions {
            viewport: Viewport::Inline(TAIL_HEIGHT),
        })?;
        Ok(Self { term })
    }

    /// Redraw the mutable tail region from app state via the `Tail` widget
    /// (`MutableLine` / `ThinkingIndicator` / `InputPrompt`).
    pub fn draw_tail(&mut self, app: &TuiApp) -> io::Result<()> {
        self.term
            .draw(|frame| render::draw_tail_frame(frame, app))?;
        Ok(())
    }

    /// Commit finalized [`RenderedLine`]s into the scrollback (never touched
    /// again). Lines are wrapped CJK-aware via the `TranscriptLine` widget; the
    /// `insert_before` area is sized to the wrapped row count (spec tui12/tui41).
    pub fn commit_to_scrollback(&mut self, lines: &[RenderedLine]) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }
        let width = crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80);
        let height = render::commit_height(lines, width);
        self.term.insert_before(height, |buf| {
            render::render_commit_lines_into_buf(lines, width, buf)
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
