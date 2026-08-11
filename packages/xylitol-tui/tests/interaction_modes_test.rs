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

    // Wheel in dock must not scroll transcript (ptim10).
    let top = scroll.scroll_top();
    let dock_wheel = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 0,
        row: 3,
        modifiers: KeyModifiers::NONE,
    };
    assert!(!sel.handle_mouse(&dock_wheel, &mut scroll, tr, dock, &mut sink));
    assert_eq!(scroll.scroll_top(), top);
}

#[test]
fn mode_b_paint_caps_at_terminal_height() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_mode_b_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: (0..20).map(|i| format!("L{i}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("paint");
    // Mode B must not grow scrollback via overflow lines.
    assert_eq!(tui.terminal.rows(), 8);
}

#[test]
fn mode_b_selection_emits_osc52_outside_batch() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 10),
        InteractionMode::ApplicationOwned,
    );
    tui.set_mode_b_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: vec![
            "hello".into(),
            "world".into(),
            "dock-a".into(),
            "dock-b".into(),
        ],
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");
    tui.terminal.clear_writes();

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let drag = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 4,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 4,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(down)),
        xylitol_tui::InputReaction::Rerender
    );
    tui.dispatch_event(InputEvent::Mouse(drag));
    tui.dispatch_event(InputEvent::Mouse(up));
    tui.request_render(false);
    tui.render_now().expect("copy paint");
    let raw = tui.terminal.all_writes();
    assert!(
        raw.contains("\x1b]52;c;"),
        "expected OSC52 clipboard write, got: {raw:?}"
    );
}

#[test]
fn mode_b_suspend_restores_alt_and_mouse() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 12),
        InteractionMode::ApplicationOwned,
    );
    tui.terminal.start();
    tui.begin_application_owned_session();
    let enter = tui.terminal.alt_enter_calls();
    let mouse = tui.terminal.mouse_enable_calls();
    tui.with_terminal_suspended(|| {
        // external editor owns TTY
    });
    assert!(tui.application_session_active());
    assert!(tui.terminal.alternate_screen_active());
    assert!(tui.mouse_capture_enabled());
    assert!(tui.terminal.alt_enter_calls() > enter);
    assert!(tui.terminal.mouse_enable_calls() > mouse);
}

#[test]
fn mode_b_wheel_scrolls_app_viewport() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 6),
        InteractionMode::ApplicationOwned,
    );
    tui.set_mode_b_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: (0..20).map(|i| format!("L{i:02}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");

    // Follow-end then wheel up should move off the bottom and stay there.
    let wheel = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(wheel)),
        xylitol_tui::InputReaction::Rerender
    );
    tui.request_render(false);
    tui.render_now().expect("after wheel");
    // Content lines L00..L17 + dock L18,L19; viewport content height=4, follow end
    // shows L14..L17. Wheel -3 → L11..L14 must persist after project_frame.
    let raw = tui.terminal.all_writes();
    assert!(
        raw.contains("L11"),
        "wheel scroll must persist across paint, got: {raw:?}"
    );
    assert!(
        !raw.contains("L17") || raw.rfind("L11").unwrap() > raw.rfind("L17").unwrap_or(0),
        "must not snap back to follow-end after wheel"
    );
}

#[test]
fn mode_b_finish_dumps_transcript_to_main_screen() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_mode_b_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: (0..10).map(|i| format!("DUMP{i}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");
    tui.terminal.clear_writes();
    tui.finish_inline();
    assert!(!tui.application_session_active());
    assert!(!tui.terminal.alternate_screen_active());
    let raw = tui.terminal.all_writes();
    assert!(
        raw.contains("DUMP0") && raw.contains("DUMP9"),
        "exit must dump transcript onto main screen, got: {raw:?}"
    );
}
