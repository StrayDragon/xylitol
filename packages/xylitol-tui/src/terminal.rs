use crossterm::terminal;
use std::io::{self, Write};

/// Terminal trait - abstract output interface for TUI.
pub trait Terminal {
    fn write(&mut self, data: &str);
    fn columns(&self) -> u16;
    fn rows(&self) -> u16;
    fn hide_cursor(&mut self);
    fn show_cursor(&mut self);
    fn clear_line(&mut self);
    fn clear_from_cursor(&mut self);
    fn clear_screen(&mut self);
    fn flush(&mut self);
}

/// Crossterm-based terminal output.
pub struct CrosstermTerminal {
    columns: u16,
    rows: u16,
}

impl CrosstermTerminal {
    pub fn new() -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        Ok(Self {
            columns: cols,
            rows,
        })
    }

    pub fn refresh_size(&mut self) {
        if let Ok((cols, rows)) = terminal::size() {
            self.columns = cols;
            self.rows = rows;
        }
    }
}

impl Terminal for CrosstermTerminal {
    fn write(&mut self, data: &str) {
        let mut stdout = io::stdout();
        let _ = stdout.write_all(data.as_bytes());
    }

    fn columns(&self) -> u16 {
        self.columns
    }

    fn rows(&self) -> u16 {
        self.rows
    }

    fn hide_cursor(&mut self) {
        let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Hide);
    }

    fn show_cursor(&mut self) {
        let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Show);
    }

    fn clear_line(&mut self) {
        let _ = io::stdout().write_all(b"\x1b[K");
    }

    fn clear_from_cursor(&mut self) {
        let _ = io::stdout().write_all(b"\x1b[J");
    }

    fn clear_screen(&mut self) {
        let _ = io::stdout().write_all(b"\x1b[2J\x1b[H");
    }

    fn flush(&mut self) {
        let _ = io::stdout().flush();
    }
}
