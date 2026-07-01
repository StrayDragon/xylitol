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
///   [streaming assistant text]   ← only while a turn streams (may wrap to
///   [  ...wrapped rows...]       ←   several rows when text exceeds width)
///   [thinking/loading indicator] ← only while a turn streams (italic, dimmed)
///   [❯ input prompt]             ← always; background block
///
/// Tall enough to show a few wrapped rows of streaming text plus the indicator
/// and input line. The full reply history lives in the scrollback above; the
/// tail only shows the in-progress tail end.
const TAIL_HEIGHT: u16 = 8;

/// Owns the inline terminal. Dropping restores raw mode + leaves scrollback.
pub struct InlineTerminal {
    term: init::DefaultTerminal,
}

impl InlineTerminal {
    /// Enable raw mode and create an inline viewport (no alt screen).
    ///
    /// Before entering, scroll the terminal so the cursor sits at the bottom
    /// row. Without this, `Viewport::Inline(N)` reserves N rows at the CURRENT
    /// cursor position — which may be mid-terminal on startup (e.g. right after
    /// the shell prompt), leaving empty rows below the viewport (fix: input box
    /// appeared to "float" above the bottom). Pushing the cursor down first
    /// guarantees the viewport anchors to the terminal bottom.
    pub fn enter() -> io::Result<Self> {
        // Print enough newlines to push the cursor to the bottom of the
        // terminal, then let ratatui's inline viewport reserve its height there.
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

    /// Redraw the mutable tail region from the app state.
    pub fn draw_tail(&mut self, app: &TuiApp) -> io::Result<()> {
        self.term
            .draw(|frame| render::draw_tail_frame(frame, app))?;
        Ok(())
    }

    /// Commit finalized lines into the scrollback (never touched again).
    ///
    /// Lines are wrapped to the terminal width BEFORE committing so long text
    /// flows to multiple physical rows instead of being truncated at the right
    /// edge. Renders each cell manually (instead of `Line::render`) to prevent
    /// ratatui's CJK filler cells from appearing as visible spaces.
    pub fn commit_to_scrollback(&mut self, lines: &[Line]) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }
        // Wrap each logical line to the terminal width (fix: text was truncated
        // at the right edge instead of wrapping). crossterm::terminal::size()
        // gives the full terminal width, which equals the inline viewport width.
        let width = crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80);
        let wrapped: Vec<Line<'static>> = lines
            .iter()
            .flat_map(|line| render::wrap_line_to_width(line, width))
            .collect();
        let height = u16::try_from(wrapped.len()).unwrap_or(u16::MAX);
        self.term.insert_before(height, |buf| {
            let area = buf.area;
            for (i, line) in wrapped.iter().enumerate() {
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
