pub const DEMO_READY_NEEDLE: &str = "theme:dark";

pub const DEMO_COMMAND_PLATE_NEEDLE: &str = "Command Plate";
pub const DEMO_SETTINGS_NEEDLE: &str = "Session Settings";

pub const PRODUCT_READY_NEEDLE: &str = " · fake";

pub const FAKE_HELLO: &str = "Hello from fake provider";

#[path = "tui_e2e/lab_record.rs"]
mod lab_record;
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

    fn blank_row(&self) -> Vec<Cell> {
        vec![Cell::default(); self.cols]
    }

    fn clear_all(&mut self) {
        let blank = self.blank_row();
        for row in self.grid.iter_mut() {
            *row = blank.clone();
        }
    }

    fn clear_from_cursor(&mut self) {
        for col in self.cursor_col..self.cols {
            self.grid[self.cursor_row][col] = Cell::default();
        }
        for row in (self.cursor_row + 1)..self.rows {
            self.grid[row] = self.blank_row();
        }
    }

    fn clear_to_cursor(&mut self) {
        for row in 0..self.cursor_row {
            self.grid[row] = self.blank_row();
        }
        for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
            self.grid[self.cursor_row][col] = Cell::default();
        }
    }

    fn clear_row(&mut self) {
        self.grid[self.cursor_row] = self.blank_row();
    }

    fn clear_row_from_cursor(&mut self) {
        for col in self.cursor_col..self.cols {
            self.grid[self.cursor_row][col] = Cell::default();
        }
    }

    fn clear_row_to_cursor(&mut self) {
        for col in 0..=self.cursor_col.min(self.cols.saturating_sub(1)) {
            self.grid[self.cursor_row][col] = Cell::default();
        }
    }

    fn scroll_up(&mut self, n: usize) {
        for _ in 0..n {
            self.grid.remove(0);
            let blank = self.blank_row();
            self.grid.push(blank);
        }
    }

    fn scroll_down(&mut self, n: usize) {
        for _ in 0..n {
            self.grid.pop();
            let blank = self.blank_row();
            self.grid.insert(0, blank);
        }
    }

    fn insert_lines(&mut self, n: usize) {
        for _ in 0..n.min(self.rows) {
            self.grid.pop();
            let blank = self.blank_row();
            self.grid.insert(self.cursor_row, blank);
        }
    }

    fn delete_lines(&mut self, n: usize) {
        for _ in 0..n.min(self.rows.saturating_sub(self.cursor_row)) {
            self.grid.remove(self.cursor_row);
            let blank = self.blank_row();
            self.grid.push(blank);
        }
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
            'J' => match first_param(params, 0).unwrap_or(0) {
                0 => self.screen.clear_from_cursor(),
                1 => self.screen.clear_to_cursor(),
                _ => self.screen.clear_all(),
            },
            'K' => match first_param(params, 0).unwrap_or(0) {
                0 => self.screen.clear_row_from_cursor(),
                1 => self.screen.clear_row_to_cursor(),
                _ => self.screen.clear_row(),
            },
            'S' => self
                .screen
                .scroll_up(first_param(params, 0).unwrap_or(1) as usize),
            'T' => self
                .screen
                .scroll_down(first_param(params, 0).unwrap_or(1) as usize),
            'L' => self
                .screen
                .insert_lines(first_param(params, 0).unwrap_or(1) as usize),
            'M' => self
                .screen
                .delete_lines(first_param(params, 0).unwrap_or(1) as usize),
            'A' => {
                let n = first_param(params, 0).unwrap_or(1) as usize;
                self.screen.cursor_row = self.screen.cursor_row.saturating_sub(n);
            }
            'B' => {
                let n = first_param(params, 0).unwrap_or(1) as usize;
                self.screen.cursor_row =
                    (self.screen.cursor_row + n).min(self.screen.rows.saturating_sub(1));
            }
            'C' => {
                let n = first_param(params, 0).unwrap_or(1) as usize;
                self.screen.cursor_col =
                    (self.screen.cursor_col + n).min(self.screen.cols.saturating_sub(1));
            }
            'D' => {
                let n = first_param(params, 0).unwrap_or(1) as usize;
                self.screen.cursor_col = self.screen.cursor_col.saturating_sub(n);
            }
            _ => {}
        }
    }
}

fn first_param(params: &Params, n: usize) -> Option<u16> {
    params.iter().nth(n).and_then(|s| s.first().copied())
}
