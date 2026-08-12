//! ApplicationOwned / dual interaction modes (c2070 / package-tui-interaction-modes).

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
fn default_tui_is_inline() {
    let tui = TUI::new(LoggingVirtualTerminal::new(40, 12));
    assert_eq!(tui.interaction_mode(), InteractionMode::Inline);
    assert!(!tui.application_session_active());
}

#[test]
fn application_owned_begin_end_records_alt_and_mouse() {
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
fn application_owned_finish_tears_down_without_leak() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 12),
        InteractionMode::ApplicationOwned,
    );
    tui.add_child(Box::new(StaticLines {
        lines: vec!["hi".into()],
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.finish();
    assert!(!tui.application_session_active());
    assert!(!tui.terminal.alternate_screen_active());
    assert!(!tui.mouse_capture_enabled());
}

#[test]
fn inline_begin_application_owned_is_noop() {
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
fn application_owned_paint_caps_at_terminal_height() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: (0..20).map(|i| format!("L{i}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("paint");
    // ApplicationOwned must not grow scrollback via overflow lines.
    assert_eq!(tui.terminal.rows(), 8);
}

#[test]
fn application_owned_dock_filling_terminal_keeps_dock_lines() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 4),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(8);
    tui.add_child(Box::new(StaticLines {
        lines: (0..8).map(|i| format!("FULL{i}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("paint");
    let raw = tui.terminal.all_writes();
    assert!(
        raw.contains("FULL4") && raw.contains("FULL7") && !raw.contains("FULL3"),
        "dock must occupy the full terminal without a synthetic transcript row: {raw:?}"
    );
}

#[test]
fn application_owned_selection_emits_osc52_outside_batch() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 10),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
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
fn application_owned_suspend_restores_alt_and_mouse() {
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
fn application_owned_suspend_restores_terminal_before_resuming_panic() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 12),
        InteractionMode::ApplicationOwned,
    );
    tui.terminal.start();
    tui.begin_application_owned_session();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tui.with_terminal_suspended(|| panic!("external process wrapper panicked"));
    }));

    assert!(result.is_err());
    assert!(tui.application_session_active());
    assert!(tui.terminal.alternate_screen_active());
    assert!(tui.mouse_capture_enabled());
}

#[test]
fn application_owned_wheel_scrolls_app_viewport() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 6),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
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
fn application_owned_copy_notice_armed_on_release_and_clears() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: vec![
            "hello world".into(),
            "second line".into(),
            "third".into(),
            "fourth".into(),
            "status".into(),
            "input".into(),
        ],
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");

    assert!(!tui.copy_notice_active());
    assert!(!tui.take_copy_notice());

    // Empty click → no notice.
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 1,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let _ = tui.dispatch_event(InputEvent::Mouse(down));
    let _ = tui.dispatch_event(InputEvent::Mouse(up));
    assert!(!tui.copy_notice_active());
    assert!(!tui.take_copy_notice());

    // Drag select → copy → notice.
    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let drag = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 5,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 5,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(down)),
        xylitol_tui::InputReaction::Rerender
    );
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(drag)),
        xylitol_tui::InputReaction::Rerender
    );
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(up)),
        xylitol_tui::InputReaction::Rerender
    );
    assert!(tui.copy_notice_active());
    assert!(tui.take_copy_notice());
    assert!(!tui.take_copy_notice());
    assert!(tui.copy_notice_active());
}

#[test]
fn application_owned_finish_dumps_transcript_to_main_screen() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: (0..10).map(|i| format!("DUMP{i}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");
    tui.terminal.clear_writes();
    tui.finish_application_owned();
    assert!(!tui.application_session_active());
    assert!(!tui.terminal.alternate_screen_active());
    let raw = tui.terminal.all_writes();
    assert!(
        raw.contains("DUMP0") && raw.contains("DUMP9"),
        "exit must dump transcript onto main screen, got: {raw:?}"
    );
}

#[test]
fn application_owned_finish_can_skip_appending_session_to_main_scrollback() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_append_session_to_main_scrollback_on_exit(false);
    tui.set_dock_rows(2);
    tui.add_child(Box::new(StaticLines {
        lines: (0..10).map(|i| format!("NODUMP{i}")).collect(),
    }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");
    tui.terminal.clear_writes();
    tui.finish_application_owned();
    assert!(!tui.application_session_active());
    let raw = tui.terminal.all_writes();
    assert!(
        !raw.contains("NODUMP0") && !raw.contains("NODUMP9"),
        "opting out must not append session lines to main scrollback, got: {raw:?}"
    );
}

#[test]
fn application_owned_finish_flushes_queued_clipboard_without_another_paint() {
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_append_session_to_main_scrollback_on_exit(false);
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.enqueue_clipboard_sequences(["\x1b]52;c;cGVuZGluZw==\x07".to_string()]);
    tui.terminal.clear_writes();

    tui.finish_application_owned();

    assert!(
        tui.terminal
            .all_writes()
            .contains("\x1b]52;c;cGVuZGluZw==\x07"),
        "finish must not drop clipboard data queued after the last paint"
    );
}

/// Focused component that records mouse Downs (simulates Editor receiving dock presses).
struct DockMouseProbe {
    downs: std::rc::Rc<std::cell::Cell<u32>>,
}

impl Component for DockMouseProbe {
    fn render(&mut self, _width: usize) -> Vec<String> {
        vec![
            "L0".into(),
            "L1".into(),
            "dock-status".into(),
            "dock-editor".into(),
        ]
    }
    fn handle_input(&mut self, event: InputEvent) {
        if let InputEvent::Mouse(m) = event
            && matches!(m.kind, MouseEventKind::Down(MouseButton::Left))
        {
            self.downs.set(self.downs.get() + 1);
        }
    }
    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        matches!(
            event,
            InputEvent::Mouse(m) if matches!(m.kind, MouseEventKind::Down(MouseButton::Left))
        )
    }
    fn invalidate(&mut self) {}
}

#[test]
fn application_owned_dock_down_falls_through_to_focused_component() {
    let downs = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    tui.add_child(Box::new(DockMouseProbe {
        downs: downs.clone(),
    }));
    tui.set_focus(Some(0));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");

    // Establish a transcript selection, then Down in dock must clear it and still
    // fall through so the focused component sees the press (ptim13).
    let tr_down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let tr_drag = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 2,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    let tr_up = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 2,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    tui.dispatch_event(InputEvent::Mouse(tr_down));
    tui.dispatch_event(InputEvent::Mouse(tr_drag));
    tui.dispatch_event(InputEvent::Mouse(tr_up));

    let dock_down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 7, // bottom dock row in 8-row terminal with dock_rows=2
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(dock_down)),
        xylitol_tui::InputReaction::Rerender
    );
    assert_eq!(
        downs.get(),
        1,
        "dock Down must fall through to focused component after clearing transcript selection"
    );
}

/// Host-style ApplicationOwned path: dock mouse → focused [`Editor`] with absolute
/// screen coords + [`Editor::set_screen_origin`] (demo / product wiring).
struct SharedEditorHost {
    editor: std::rc::Rc<std::cell::RefCell<xylitol_tui::Editor>>,
    term_rows: u16,
    dock_rows: usize,
}

impl Component for SharedEditorHost {
    fn render(&mut self, width: usize) -> Vec<String> {
        // Pad transcript above the dock so dock fallthrough is exercised.
        let mut lines: Vec<String> = (0..self.term_rows as usize)
            .map(|i| format!("transcript-{i}"))
            .collect();
        let dock = self.dock_rows.min(lines.len());
        let editor_lines = self.editor.borrow_mut().render(width);
        let start = lines.len().saturating_sub(dock);
        for (i, line) in editor_lines.into_iter().take(dock).enumerate() {
            if start + i < lines.len() {
                lines[start + i] = line;
            }
        }
        lines
    }

    fn handle_input(&mut self, event: InputEvent) {
        if let InputEvent::Mouse(mouse) = event {
            let (origin_row, origin_col) =
                xylitol_tui::editor_screen_origin(self.term_rows, self.dock_rows, 0);
            self.editor
                .borrow_mut()
                .set_screen_origin(origin_row, origin_col);
            self.editor
                .borrow_mut()
                .handle_input(InputEvent::Mouse(mouse));
        }
    }

    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        self.editor.borrow().input_wants_rerender(event)
    }

    fn invalidate(&mut self) {
        self.editor.borrow_mut().invalidate();
    }
}

#[test]
fn application_owned_editor_double_click_word_via_dock_fallthrough() {
    use xylitol_tui::{
        Clock, Editor, EditorOptions, EditorTheme, SystemClock, editor_screen_origin,
    };

    let term_rows = 12u16;
    let dock_rows = 4usize;
    let editor = std::rc::Rc::new(std::cell::RefCell::new(Editor::new(
        EditorTheme::default(),
        EditorOptions {
            padding_x: 0,
            terminal_rows: term_rows as usize,
        },
        Box::new(SystemClock) as Box<dyn Clock>,
    )));
    editor.borrow_mut().set_text("say apple pie".into());
    let _ = editor.borrow_mut().render(40);

    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, term_rows),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(dock_rows);
    tui.add_child(Box::new(SharedEditorHost {
        editor: editor.clone(),
        term_rows,
        dock_rows,
    }));
    tui.set_focus(Some(0));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");

    let (origin_row, _) = editor_screen_origin(term_rows, dock_rows, 0);
    // Editor local content row 1 → absolute screen row origin+1; col 7 = 'l' of apple.
    let screen_row = origin_row + 1;
    let col = 7u16;
    let click = |kind| MouseEvent {
        kind,
        column: col,
        row: screen_row,
        modifiers: KeyModifiers::NONE,
    };

    // First click (character) — no copy.
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(click(MouseEventKind::Down(
            MouseButton::Left
        )))),
        xylitol_tui::InputReaction::Rerender
    );
    let _ = tui.dispatch_event(InputEvent::Mouse(click(MouseEventKind::Up(
        MouseButton::Left,
    ))));

    // Second click — whole word.
    let _ = tui.dispatch_event(InputEvent::Mouse(click(MouseEventKind::Down(
        MouseButton::Left,
    ))));
    assert_eq!(
        editor.borrow().selected_text().as_deref(),
        Some("apple"),
        "double-click Down must expand to whole word before release"
    );
    let _ = tui.dispatch_event(InputEvent::Mouse(click(MouseEventKind::Up(
        MouseButton::Left,
    ))));
    assert_eq!(editor.borrow().selected_text().as_deref(), Some("apple"));
    assert!(
        editor
            .borrow_mut()
            .take_pending_clipboard()
            .iter()
            .any(|s| s.contains("\x1b]52;")),
        "editor double-click copy-on-release must queue OSC52"
    );
}
