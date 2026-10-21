use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
};
use crossterm::{cursor, execute};
use std::io::{self, Write};
use std::time::{Duration, Instant};

use crate::keys::set_kitty_protocol_active;

const KITTY_FLAGS_REQUEST: u8 = 7;

const KITTY_POP_ALL_SEQUENCE: &str = "\x1b[<8u";
const KITTY_KEYBOARD_PROTOCOL_QUERY: &str = "\x1b[?u\x1b[c";

const MODIFY_OTHER_KEYS_ENABLE: &str = "\x1b[>4;2m";
const MODIFY_OTHER_KEYS_DISABLE: &str = "\x1b[>4;0m";

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
    fn refresh_size(&mut self) {}

    fn set_size_hint(&mut self, _cols: u16, _rows: u16) {}

    fn start(&mut self) {}
    fn stop(&mut self) {}

    fn enable_mouse_capture(&mut self) {}
    fn disable_mouse_capture(&mut self) {}
    fn mouse_capture_active(&self) -> bool {
        false
    }

    fn enter_alternate_screen(&mut self) {}
    fn leave_alternate_screen(&mut self) {}
    fn alternate_screen_active(&self) -> bool {
        false
    }

    fn set_title(&mut self, _title: &str) {}
    fn set_progress(&mut self, _active: bool) {}
    fn move_by(&mut self, _lines: i32) {}
}

#[derive(Debug, Clone, Copy)]
pub struct NullTerminal {
    cols: u16,
    rows: u16,
}

impl NullTerminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

impl Default for NullTerminal {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

impl Terminal for NullTerminal {
    fn write(&mut self, _data: &str) {}
    fn columns(&self) -> u16 {
        self.cols
    }
    fn rows(&self) -> u16 {
        self.rows
    }
    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn clear_line(&mut self) {}
    fn clear_from_cursor(&mut self) {}
    fn clear_screen(&mut self) {}
    fn flush(&mut self) {}
}

pub fn env_requests_mouse_capture() -> bool {
    match std::env::var("XYLITOL_TUI_MOUSE") {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

pub fn parse_kitty_flags(seq: &str) -> Option<u32> {
    let rest = seq.strip_prefix("\x1b[?")?;
    let digits = rest.strip_suffix('u')?;
    digits.parse::<u32>().ok()
}

pub struct CrosstermTerminal {
    columns: u16,
    rows: u16,
    kitty_pushed: bool,
    modify_other_keys_active: bool,
    started: bool,
    mouse_capture_desired: bool,
    mouse_capture_active: bool,
    alternate_screen_active: bool,
}

impl Default for CrosstermTerminal {
    fn default() -> Self {
        Self::new().expect("CrosstermTerminal::new requires a tty")
    }
}

impl CrosstermTerminal {
    pub fn new() -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        Ok(Self {
            columns: cols,
            rows,
            kitty_pushed: false,
            modify_other_keys_active: false,
            started: false,
            mouse_capture_desired: false,
            mouse_capture_active: false,
            alternate_screen_active: false,
        })
    }

    pub fn refresh_size_impl(&mut self) {
        if let Ok((cols, rows)) = terminal::size() {
            self.columns = cols;
            self.rows = rows;
        }
    }

    fn negotiate_keyboard_protocol(&mut self) {
        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::from_bits_truncate(
                KITTY_FLAGS_REQUEST,
            ))
        );
        self.write_raw(KITTY_KEYBOARD_PROTOCOL_QUERY);
        set_kitty_protocol_active(true);
        self.kitty_pushed = true;
    }

    fn rearm_keyboard_after_alt_screen(&mut self) {
        let _ = execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::from_bits_truncate(
                KITTY_FLAGS_REQUEST,
            ))
        );
        set_kitty_protocol_active(true);
        self.kitty_pushed = true;
        self.write_raw(MODIFY_OTHER_KEYS_ENABLE);
        self.modify_other_keys_active = true;
    }

    fn pop_keyboard_enhancements(&mut self) {
        self.write_raw(KITTY_POP_ALL_SEQUENCE);
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        set_kitty_protocol_active(false);
        self.kitty_pushed = false;
    }

    fn disable_modify_other_keys(&mut self) {
        if !self.modify_other_keys_active {
            return;
        }
        self.write_raw(MODIFY_OTHER_KEYS_DISABLE);
        self.modify_other_keys_active = false;
    }

    fn write_raw(&mut self, data: &str) {
        let _ = io::stdout().write_all(data.as_bytes());
    }

    fn drain_input(&mut self, max_ms: u64, idle_ms: u64) {
        let deadline = Instant::now() + Duration::from_millis(max_ms);
        let mut last_seen = Instant::now();
        while Instant::now() < deadline {
            let idle_deadline = last_seen + Duration::from_millis(idle_ms);
            if Instant::now() >= idle_deadline {
                break;
            }
            if crossterm::event::poll(Duration::from_millis(idle_ms)).unwrap_or(false) {
                let _ = crossterm::event::read();
                last_seen = Instant::now();
            }
        }
    }

    fn sync_mouse_capture(&mut self) {
        if !self.started {
            return;
        }
        if self.mouse_capture_desired && !self.mouse_capture_active {
            let _ = execute!(io::stdout(), EnableMouseCapture);
            self.mouse_capture_active = true;
        } else if !self.mouse_capture_desired && self.mouse_capture_active {
            let _ = execute!(io::stdout(), DisableMouseCapture);
            self.mouse_capture_active = false;
        }
    }

    fn release_mouse_capture_active(&mut self) {
        if self.mouse_capture_active {
            let _ = execute!(io::stdout(), DisableMouseCapture);
            self.mouse_capture_active = false;
        }
    }
}

impl Terminal for CrosstermTerminal {
    fn write(&mut self, data: &str) {
        self.write_raw(data);
    }

    fn columns(&self) -> u16 {
        self.columns
    }

    fn rows(&self) -> u16 {
        self.rows
    }

    fn hide_cursor(&mut self) {
        let _ = execute!(io::stdout(), cursor::Hide);
    }

    fn show_cursor(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show);
    }

    fn clear_line(&mut self) {
        let _ = execute!(io::stdout(), Clear(ClearType::UntilNewLine));
    }

    fn clear_from_cursor(&mut self) {
        let _ = execute!(io::stdout(), Clear(ClearType::FromCursorDown));
    }

    fn clear_screen(&mut self) {
        let _ = execute!(io::stdout(), Clear(ClearType::All), cursor::MoveTo(0, 0));
    }

    fn flush(&mut self) {
        let _ = io::stdout().flush();
    }

    fn refresh_size(&mut self) {
        self.refresh_size_impl();
    }

    fn set_size_hint(&mut self, cols: u16, rows: u16) {
        if cols > 0 {
            self.columns = cols;
        }
        if rows > 0 {
            self.rows = rows;
        }
    }

    fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;
        let _ = terminal::enable_raw_mode();
        let _ = execute!(io::stdout(), EnableBracketedPaste);
        self.negotiate_keyboard_protocol();
        self.sync_mouse_capture();
    }

    fn stop(&mut self) {
        if !self.started {
            return;
        }
        self.started = false;

        self.release_mouse_capture_active();

        if self.alternate_screen_active {
            self.pop_keyboard_enhancements();
            self.disable_modify_other_keys();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            self.alternate_screen_active = false;
        }

        let _ = execute!(io::stdout(), DisableBracketedPaste);

        self.pop_keyboard_enhancements();
        self.disable_modify_other_keys();

        self.drain_input(200, 30);

        let _ = terminal::disable_raw_mode();
        self.show_cursor();
        self.flush();
    }

    fn enable_mouse_capture(&mut self) {
        self.mouse_capture_desired = true;
        self.sync_mouse_capture();
    }

    fn disable_mouse_capture(&mut self) {
        self.mouse_capture_desired = false;
        self.sync_mouse_capture();
    }

    fn mouse_capture_active(&self) -> bool {
        self.mouse_capture_active
    }

    fn enter_alternate_screen(&mut self) {
        if self.alternate_screen_active {
            return;
        }
        let _ = execute!(io::stdout(), EnterAlternateScreen);
        self.alternate_screen_active = true;
        if self.started {
            let _ = execute!(io::stdout(), EnableBracketedPaste);
            self.rearm_keyboard_after_alt_screen();
        }
    }

    fn leave_alternate_screen(&mut self) {
        if !self.alternate_screen_active {
            return;
        }
        self.pop_keyboard_enhancements();
        self.disable_modify_other_keys();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        self.alternate_screen_active = false;
    }

    fn alternate_screen_active(&self) -> bool {
        self.alternate_screen_active
    }

    fn set_title(&mut self, title: &str) {
        let _ = execute!(io::stdout(), SetTitle(title));
    }

    fn set_progress(&mut self, active: bool) {
        if active {
            self.write_raw("\x1b]9;4;3\x07");
        } else {
            self.write_raw("\x1b]9;4;0\x07");
        }
    }

    fn move_by(&mut self, lines: i32) {
        if lines > 0 {
            let _ = execute!(io::stdout(), cursor::MoveDown(lines as u16));
        } else if lines < 0 {
            let _ = execute!(io::stdout(), cursor::MoveUp((-lines) as u16));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nonzero_flags() {
        assert_eq!(parse_kitty_flags("\x1b[?7u"), Some(7));
        assert_eq!(parse_kitty_flags("\x1b[?1u"), Some(1));
    }

    #[test]
    fn parse_zero_flags() {
        assert_eq!(parse_kitty_flags("\x1b[?0u"), Some(0));
    }

    #[test]
    fn parse_nonmatch_device_attributes() {
        assert_eq!(parse_kitty_flags("\x1b[?62;4;52c"), None);
    }

    #[test]
    fn parse_garbage() {
        assert_eq!(parse_kitty_flags("abc"), None);
        assert_eq!(parse_kitty_flags(""), None);
        assert_eq!(parse_kitty_flags("\x1b[7u"), None);
        assert_eq!(parse_kitty_flags("\x1b[?7"), None);
    }
}
