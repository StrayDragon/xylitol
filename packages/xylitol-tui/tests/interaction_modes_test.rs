//! Mode B / dual interaction modes (c2070 / package-tui-interaction-modes).

mod support;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use support::LoggingVirtualTerminal;
use xylitol_tui::{
    Component, InputEvent, InteractionMode, RecordingClipboardSink, ScreenRect, ScrollView,
    SelectionController, TUI, Terminal,
};

struct StaticLines {
    lines: Vec<String>,
}

impl Component for StaticLines {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.lines.clone()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
}

#[test]
fn default_tui_is_mode_a_inline() {
    let tui = TUI::new(LoggingVirtualTerminal::new(40, 12));
    assert_eq!(tui.interaction_mode(), InteractionMode::Inline);
    assert!(!tui.application_session_active());
}

#[test]
fn mode_b_begin_end_records_alt_and_mouse() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 12),
        InteractionMode::ApplicationOwned,
    );
    tui.terminal.start();
    tui.begin_application_owned_session();
    assert!(tui.application_session_active());
    assert!(tui.terminal.alternate_screen_active());
    assert!(tui.mouse_capture_enabled());
    assert!(tui.terminal.alt_enter_calls() >= 1);
    assert!(tui.terminal.mouse_enable_calls() >= 1);

    tui.end_application_owned_session();
    assert!(!tui.application_session_active());
    assert!(!tui.terminal.alternate_screen_active());
    assert!(!tui.mouse_capture_enabled());
    assert!(tui.terminal.alt_leave_calls() >= 1);
    assert!(tui.terminal.mouse_disable_calls() >= 1);
}

#[test]
fn mode_b_finish_inline_tears_down_without_leak() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 12),
        InteractionMode::ApplicationOwned,
    );
    tui.add_child(Box::new(StaticLines {
        lines: vec!["hi".into()],
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.finish_inline();
    assert!(!tui.application_session_active());
    assert!(!tui.terminal.alternate_screen_active());
    assert!(!tui.mouse_capture_enabled());
}

#[test]
fn mode_a_begin_application_is_noop() {
    let mut tui = TUI::new(LoggingVirtualTerminal::new(40, 12));
    tui.terminal.start();
    tui.begin_application_owned_session();
    assert!(!tui.application_session_active());
    assert_eq!(tui.terminal.alt_enter_calls(), 0);
}

#[test]
fn selection_scroll_and_dock_exclude_integration() {
    let mut scroll = ScrollView::new(3);
    scroll.set_lines(vec![
        "one".into(),
        "two".into(),
        "three".into(),
        "four".into(),
    ]);
    let mut sel = SelectionController::new();
    let mut sink = RecordingClipboardSink::default();
    let tr = ScreenRect {
        row: 0,
        col: 0,
        height: 3,
        width: 20,
    };
    let dock = ScreenRect {
        row: 3,
        col: 0,
        height: 2,
        width: 20,
    };
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let drag = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 3,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 3,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    sel.handle_mouse(&down, &mut scroll, tr, dock, &mut sink);
    sel.handle_mouse(&drag, &mut scroll, tr, dock, &mut sink);
    sel.handle_mouse(&up, &mut scroll, tr, dock, &mut sink);
    assert_eq!(sink.copies, vec!["one".to_string()]);

    // Pointer in dock must not start selection.
    let dock_down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 3,
        modifiers: KeyModifiers::NONE,
    };
    sel.clear();
    sink.copies.clear();
    assert!(!sel.handle_mouse(&dock_down, &mut scroll, tr, dock, &mut sink));
}
