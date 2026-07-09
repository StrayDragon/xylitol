//! Host harness tests — no real TTY (ath5).

use super::host::{HostEvent, HostSession, LayoutMode, TOO_SMALL_HINT, is_too_small};
use super::ui_root::build_root;
use xylitol_tui::{InputEvent, Terminal};

/// Minimal in-memory terminal for host tests.
struct TestTerminal {
    cols: u16,
    rows: u16,
    pub frames: Vec<String>,
    started: bool,
    stopped: bool,
}

impl TestTerminal {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            frames: Vec::new(),
            started: false,
            stopped: false,
        }
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
    }
}

#[test]
fn too_small_predicate() {
    assert!(is_too_small(39, 24));
    assert!(is_too_small(80, 5));
    assert!(!is_too_small(40, 6));
}

#[test]
fn harness_min_size_shows_hint() {
    let mut session = HostSession::new(TestTerminal::new(20, 3), build_root);
    assert_eq!(session.mode(), LayoutMode::TooSmall);
    session.render_now().expect("render");
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains(TOO_SMALL_HINT),
        "expected hint in writes, got: {joined:?}"
    );
}

#[test]
fn harness_resize_to_ready() {
    let mut session = HostSession::new(TestTerminal::new(20, 3), build_root);
    assert_eq!(session.mode(), LayoutMode::TooSmall);
    session
        .step(HostEvent::Resize { cols: 80, rows: 24 })
        .unwrap();
    session.render_now().unwrap();
    assert_eq!(session.mode(), LayoutMode::Ready);
    let joined = session.tui.terminal.frames.concat();
    assert!(
        joined.contains("transcript") || joined.contains("esc abort"),
        "expected UI chrome, got: {joined:?}"
    );
}

#[test]
fn harness_never_calls_terminal_start() {
    let mut session = HostSession::new(TestTerminal::new(80, 24), build_root);
    session.render_now().unwrap();
    session
        .step(HostEvent::Input(InputEvent::Paste("hi".into())))
        .unwrap();
    assert!(
        !session.tui.terminal.started,
        "HostSession must not call Terminal::start"
    );
}

#[test]
fn product_tui_source_has_no_tui_start_call() {
    let sources = [
        ("mod.rs", include_str!("mod.rs")),
        ("host.rs", include_str!("host.rs")),
        ("ui_root.rs", include_str!("ui_root.rs")),
        ("terminal_guard.rs", include_str!("terminal_guard.rs")),
    ];
    for (name, src) in sources {
        for line in src.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("//!") || trimmed.starts_with('*') {
                continue;
            }
            assert!(
                !trimmed.contains("TUI::start")
                    && !trimmed.contains(".start_with_flag")
                    && !trimmed.contains("tui.start("),
                "{name}: product must not call TUI::start — line: {trimmed}"
            );
        }
    }
}

#[test]
fn quit_event_stops_session() {
    let mut session = HostSession::new(TestTerminal::new(80, 24), build_root);
    session.step(HostEvent::Quit).unwrap();
    assert!(session.should_quit());
}

#[test]
fn build_root_ready_is_component() {
    let mut kids = build_root(LayoutMode::Ready);
    assert_eq!(kids.len(), 1);
    let lines = kids[0].render(80);
    assert!(!lines.is_empty());
}

fn ctrl_c_event() -> InputEvent {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    InputEvent::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn harness_ctrl_c_clears_editor_then_quits() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("keep me");

    session.step(HostEvent::Input(ctrl_c_event())).unwrap();
    assert!(
        !session.should_quit(),
        "non-empty editor must clear, not quit"
    );
    assert!(
        root.borrow().editor_text().is_empty(),
        "Ctrl+C should clear editor"
    );

    session.step(HostEvent::Input(ctrl_c_event())).unwrap();
    assert!(
        session.should_quit(),
        "second Ctrl+C on empty editor should quit"
    );
}

#[test]
fn harness_ctrl_c_consumed_before_editor_insert() {
    let mut session = HostSession::new_product_ui(TestTerminal::new(80, 24));
    let root = session.ui_root().expect("product ui").clone();
    root.borrow_mut().set_editor_text("x");
    session.step(HostEvent::Input(ctrl_c_event())).unwrap();
    assert_eq!(root.borrow().editor_text(), "");
    assert!(!session.should_quit());
}
