pub const DEMO_READY_NEEDLE: &str = "theme:dark";

pub const DEMO_COMMAND_PLATE_NEEDLE: &str = "Command Plate";
pub const DEMO_SETTINGS_NEEDLE: &str = "Session Settings";

pub const PRODUCT_READY_NEEDLE: &str = " · fake";

pub const FAKE_HELLO: &str = "Hello from fake provider";

#[path = "tui_e2e/pty.rs"]
mod pty;
#[path = "tui_e2e/tmux.rs"]
mod tmux;

use vte::{Params, Perform};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub bold: bool,
    pub fg_red: bool,
}

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

    pub fn feed(&mut self, bytes: &[u8]) {
        let mut parser = vte::Parser::new();
        let mut performer = Performer { screen: self };
        parser.advance(&mut performer, bytes);
    }

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

    pub fn text(&self) -> String {
        self.viewport().join("\n")
    }

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
