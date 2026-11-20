use xylitol_tui::{InputEvent, Terminal};

pub struct TestTerminal {
    cols: u16,
    rows: u16,
    frames: Vec<String>,
    started: bool,
    pub(crate) stopped: bool,
    mouse_capture_active: bool,
    alternate_screen_active: bool,
}

impl TestTerminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            frames: Vec::new(),
            started: false,
            stopped: false,
            mouse_capture_active: false,
            alternate_screen_active: false,
        }
    }

    pub fn frames(&self) -> &[String] {
        &self.frames
    }

    pub fn clear_frames(&mut self) {
        self.frames.clear();
    }

    pub fn started(&self) -> bool {
        self.started
    }
}

impl Terminal for TestTerminal {
    fn write(&mut self, data: &str) {
        self.frames.push(data.to_string());
    }
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
    fn set_size_hint(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
    }
    fn start(&mut self) {
        self.started = true;
    }
    fn stop(&mut self) {
        self.stopped = true;
        self.mouse_capture_active = false;
        self.alternate_screen_active = false;
    }
    fn enable_mouse_capture(&mut self) {
        self.mouse_capture_active = true;
    }
    fn disable_mouse_capture(&mut self) {
        self.mouse_capture_active = false;
    }
    fn mouse_capture_active(&self) -> bool {
        self.mouse_capture_active
    }
    fn enter_alternate_screen(&mut self) {
        self.alternate_screen_active = true;
    }
    fn leave_alternate_screen(&mut self) {
        self.alternate_screen_active = false;
    }
    fn alternate_screen_active(&self) -> bool {
        self.alternate_screen_active
    }
}

pub fn key_event(
    code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) -> InputEvent {
    InputEvent::Key(crossterm::event::KeyEvent {
        code,
        modifiers,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    })
}

#[cfg(test)]
pub(crate) fn footer_line(frame: &[String]) -> String {
    let plain: Vec<String> = frame
        .iter()
        .map(|line| xylitol_tui::utils::strip_ansi_codes(line))
        .collect();
    plain
        .get(plain.len().saturating_sub(2))
        .cloned()
        .unwrap_or_default()
}

pub fn enter_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

pub fn alt_enter_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::ALT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

pub fn esc_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}
