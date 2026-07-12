//! End-to-end TUI integration tests (c405 layer 5).
//!
//! Two drivers, non-overlapping:
//! - `pty`: portable-pty spawns the real binary, injects keys via the writer
//!   handle, reads bytes back, feeds them to a vte cell-grid oracle. Precise
//!   and fast (in-process parsing); covers crossterm's real event parsing of
//!   multi-byte sequences (Ctrl/Alt+arrow, bracketed paste).
//! - `tmux`: a hand-written `tmux` wrapper (no binding crate) drives a real
//!   terminal emulator via `send-keys` / `capture-pane -e -p`. Slower and
//!   coarser; covers real-terminal compatibility + SGR color regression.
//!
//! All cases are `#[ignore]`-gated (spec `test-infra` r8): they do not run
//! under the default `cargo test` and require the dedicated justfile target
//! `test-tui-e2e`. tmux cases additionally skip unless the `tmux` binary is
//! present and `TERM=xterm-256color` is set.
//!
//! Ready-probe SSOT for `agent_demo`: see `packages/xylitol-tui/AGENTS.md`
//! 「验证」; needle is [`DEMO_READY_NEEDLE`].
//! Product binary smoke (c485 avs2): [`PRODUCT_READY_NEEDLE`] + [`FAKE_HELLO`].

/// Substring that appears once `agent_demo` has painted a stable frame.
/// Prefer footer text (`theme:dark`) over header banners — tall seed
/// transcripts scroll the header out of the captured viewport.
pub const DEMO_READY_NEEDLE: &str = "theme:dark";

/// Overlay / plate titles visible in `agent_demo` (keep in sync with demo render).
pub const DEMO_COMMAND_PLATE_NEEDLE: &str = "Command Plate";
pub const DEMO_SETTINGS_NEEDLE: &str = "Session Settings";

/// Product TUI footer once Fake model is selected (`cwd · fake`).
pub const PRODUCT_READY_NEEDLE: &str = " · fake";

/// Fake provider default assistant text (cross-process; no thread-local scripting).
pub const FAKE_HELLO: &str = "Hello from fake provider";

#[path = "tui_e2e/pty.rs"]
mod pty;
#[path = "tui_e2e/tmux.rs"]
mod tmux;

// ── shared cell-grid oracle for both drivers ──────────────────────────────
// A minimal vte-backed screen that both PTY bytes and `tmux capture-pane -e`
// output feed into. Self-contained here (design decision 3: a local minimal
// parser over polluting the library with a `test-support` feature).

use vte::{Params, Perform};

/// One captured cell: the printed char plus the SGR attributes in effect.
/// Subset sufficient for E2E assertions (text + bold/color presence).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub bold: bool,
    pub fg_red: bool,
}

/// A minimal terminal that parses ANSI bytes into a cell grid. Enough to
/// assert "the screen contains X" and "cell (r,c) is bold/red" for smoke and
/// color-regression cases — NOT a full emulator (no scroll regions, no alt
/// screen, no Kitty graphics).
pub struct CapturedScreen {
    cols: usize,
    rows: usize,
    grid: Vec<Vec<Cell>>,
    cursor_row: usize,
    cursor_col: usize,
    bold: bool,
    fg_red: bool,
}

impl CapturedScreen {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            grid: vec![vec![Cell::default(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            bold: false,
            fg_red: false,
        }
    }

    /// Feed raw bytes (from a PTY reader or `tmux capture-pane -e -p`).
    pub fn feed(&mut self, bytes: &[u8]) {
        let mut parser = vte::Parser::new();
        let mut performer = Performer { screen: self };
        for &b in bytes {
            parser.advance(&mut performer, b);
        }
    }

    /// Visible viewport as trimmed plain-text rows.
    pub fn viewport(&self) -> Vec<String> {
        self.grid
            .iter()
            .map(|row| {
                let s: String = row
                    .iter()
                    .map(|c| if c.ch == '\0' { ' ' } else { c.ch })
                    .collect();
                s.trim_end().to_string()
            })
            .collect()
    }

    /// Concatenated viewport text (for `contains` assertions).
    pub fn text(&self) -> String {
        self.viewport().join("\n")
    }

    /// True if any cell in the viewport carries the SGR red attribute.
    pub fn any_red(&self) -> bool {
        self.grid.iter().flatten().any(|c| c.fg_red)
    }
}

struct Performer<'a> {
    screen: &'a mut CapturedScreen,
}

impl Perform for Performer<'_> {
    fn print(&mut self, c: char) {
        if self.screen.cursor_row < self.screen.rows && self.screen.cursor_col < self.screen.cols {
            self.screen.grid[self.screen.cursor_row][self.screen.cursor_col] = Cell {
                ch: c,
                bold: self.screen.bold,
                fg_red: self.screen.fg_red,
            };
        }
        self.screen.cursor_col =
            (self.screen.cursor_col + 1).min(self.screen.cols.saturating_sub(1));
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0B | 0x0C => {
                self.screen.cursor_row =
                    (self.screen.cursor_row + 1).min(self.screen.rows.saturating_sub(1));
            }
            b'\r' => self.screen.cursor_col = 0,
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, byte: char) {
        match byte {
            'H' | 'f' => {
                let row = first_param(params, 0).unwrap_or(1) as usize;
                let col = first_param(params, 1).unwrap_or(1) as usize;
                self.screen.cursor_row = row
                    .saturating_sub(1)
                    .min(self.screen.rows.saturating_sub(1));
                self.screen.cursor_col = col
                    .saturating_sub(1)
                    .min(self.screen.cols.saturating_sub(1));
            }
            'm' => {
                for p in params.iter().flat_map(|s| s.iter().copied()) {
                    match p {
                        0 => {
                            self.screen.bold = false;
                            self.screen.fg_red = false;
                        }
                        1 => self.screen.bold = true,
                        22 => self.screen.bold = false,
                        31 => self.screen.fg_red = true,
                        39 => self.screen.fg_red = false,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn first_param(params: &Params, n: usize) -> Option<u16> {
    params.iter().nth(n).and_then(|s| s.first().copied())
}
