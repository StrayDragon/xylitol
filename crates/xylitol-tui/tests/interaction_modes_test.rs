//! ApplicationOwned / dual interaction modes (c2070 / package-tui-interaction-modes).

mod support;

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use support::LoggingVirtualTerminal;
use xylitol_tui::{
    Component, Hit, InputEvent, InteractionMode, PointerClickPhase, RecordingClipboardSink,
    ScreenRect, ScrollView, SelectionController, TUI, Terminal,
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
    assert!(
        tui.try_render().expect("after wheel"),
        "wheel-only AO reproject must bypass the full-frame throttle"
    );
    assert!(tui.last_render_perf().ao_reprojected);
    assert!(
        tui.last_render_perf().ao_vertical_shifted,
        "pure wheel viewport motion should reuse terminal rows"
    );
    // Content L00..L17 + dock L18,L19; viewport content h=4.
    // Follow end shows L14..L17; one three-row notch → L11..L14.
    assert_eq!(tui.application_owned_wheel_notch(), 3);
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
fn application_owned_wheel_coalesce_applies_full_delta_without_tick_drain() {
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
    tui.render_now().expect("seed");
    tui.terminal.clear_writes();

    let frames_before = tui.frame_count();
    let notch = tui.application_owned_wheel_notch();
    assert!(tui.application_owned_scroll_by(-(notch * 2)));
    assert!(
        tui.try_render().expect("coalesced wheel paint"),
        "coalesced wheel must paint immediately"
    );
    assert_eq!(tui.frame_count(), frames_before + 1);
    assert!(tui.last_render_perf().ao_reprojected);
    assert!(
        tui.terminal.all_writes().contains("L08"),
        "both coalesced three-row wheel events must be visible in the first paint"
    );
    assert!(
        !tui.idle_tick(),
        "normal wheel motion must not continue through the edge-drag tick path"
    );
    assert!(!tui.is_render_requested());
}

#[test]
fn application_owned_wheel_first_paint_is_immediate_then_continuous_stream_is_capped() {
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
    tui.render_now().expect("seed full frame");

    let notch = tui.application_owned_wheel_notch();
    assert!(tui.application_owned_scroll_by(-notch));
    assert!(
        tui.try_render().expect("first wheel paint"),
        "the first wheel reproject must not inherit the full-frame throttle"
    );

    assert!(tui.application_owned_scroll_by(-notch));
    assert!(
        !tui.try_render().expect("continuous wheel cadence"),
        "continuous wheel paints should stay capped near 60fps"
    );
    assert!(tui.is_render_requested());

    tui.render_now().expect("settle pending viewport");
    assert!(tui.terminal.all_writes().contains("L08"));
    assert!(!tui.is_render_requested());
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

    fn clear_pointer_selection(&mut self) -> bool {
        Component::clear_pointer_selection(&mut *self.editor.borrow_mut())
    }

    fn wants_pointer_motion(&self) -> bool {
        self.editor.borrow().is_selection_dragging()
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

#[test]
fn application_owned_editor_and_transcript_selection_are_mutex() {
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
    let editor_row = origin_row + 1;
    let mouse = |kind, column, row| MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    };

    // Select "apple" in the editor (cols 4..9 on content row).
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        4,
        editor_row,
    )));
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        9,
        editor_row,
    )));
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Up(MouseButton::Left),
        9,
        editor_row,
    )));
    assert_eq!(editor.borrow().selected_text().as_deref(), Some("apple"));
    assert!(!tui.application_owned_has_selection());

    // Click + drag in transcript — editor highlight must drop; transcript owns it.
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        0,
        0,
    )));
    assert!(
        !editor.borrow().has_selection(),
        "transcript Left Down must clear editor selection"
    );
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        8,
        0,
    )));
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Up(MouseButton::Left),
        8,
        0,
    )));
    assert!(tui.application_owned_has_selection());
    assert!(!editor.borrow().has_selection());

    // Click in editor — transcript selection clears; editor can own a new range.
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Down(MouseButton::Left),
        4,
        editor_row,
    )));
    assert!(
        !tui.application_owned_has_selection(),
        "dock Left Down must clear transcript selection"
    );
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        9,
        editor_row,
    )));
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse(
        MouseEventKind::Up(MouseButton::Left),
        9,
        editor_row,
    )));
    assert_eq!(editor.borrow().selected_text().as_deref(), Some("apple"));
    assert!(!tui.application_owned_has_selection());
}

#[test]
fn application_owned_transcript_hit_priority_swallows_press() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let hit = std::sync::Arc::new(AtomicBool::new(false));
    let hit_flag = hit.clone();

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
    tui.set_transcript_hit_priority(Some(Box::new(move |col, row| {
        if row == 0 && col <= 4 {
            hit_flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    })));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");

    let down = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 1,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(down)),
        xylitol_tui::InputReaction::Rerender
    );
    assert!(hit.load(Ordering::SeqCst), "hit priority must run");
    assert!(
        !tui.application_owned_has_selection(),
        "swallowed press must not start transcript selection"
    );

    // Hook must still be installed after the dispatch (park/restore).
    hit.store(false, Ordering::SeqCst);
    let _ = tui.dispatch_event(InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 2,
        row: 0,
        modifiers: KeyModifiers::NONE,
    }));
    assert!(
        hit.load(Ordering::SeqCst),
        "hit priority must survive across dispatches"
    );
}

#[test]
fn application_owned_finalize_only_paint_lines() {
    // Long component output must not pay finalize/visible_width on every content
    // line — only the projected ≤height paint surface.
    let lines: Vec<String> = (0..200).map(|i| format!("row-{i:03} payload")).collect();
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(80, 24),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(4);
    tui.add_child(Box::new(StaticLines { lines }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.clear_finalize_counters_for_test();
    tui.render_frame().expect("ao paint");
    let perf = tui.last_render_perf();
    assert_eq!(perf.component_lines, 200);
    assert!(
        perf.paint_lines <= 24,
        "paint_lines={} must be ≤ term rows",
        perf.paint_lines
    );
    assert!(
        perf.finalize_width_checks <= perf.paint_lines as u64,
        "finalize_checks={} paint_lines={}",
        perf.finalize_width_checks,
        perf.paint_lines
    );
    assert!(
        tui.finalize_width_checks_for_test() < 50,
        "must not finalize full 200-line transcript"
    );
}

#[test]
fn application_owned_click_stays_aligned_after_repeated_wheel_scrolls() {
    let lines: Vec<String> = (0..80).map(|i| format!("L{i:02}")).collect();
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 12),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    tui.add_child(Box::new(StaticLines { lines }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.render_frame().expect("warm");

    let wheel = |kind| {
        InputEvent::Mouse(MouseEvent {
            kind,
            column: 2,
            row: 4,
            modifiers: KeyModifiers::NONE,
        })
    };
    for _ in 0..6 {
        let _ = tui.dispatch_event(wheel(MouseEventKind::ScrollUp));
        tui.render_frame().expect("wheel up");
    }
    for _ in 0..2 {
        let _ = tui.dispatch_event(wheel(MouseEventKind::ScrollDown));
        tui.render_frame().expect("wheel down");
    }
    assert!(tui.last_render_perf().ao_vertical_shifted);
    let wheel_writes = tui.terminal.all_writes();
    assert!(
        wheel_writes.contains("\x1b[3L") && wheel_writes.contains("\x1b[3M"),
        "wheel shifts should use IL/DL in both directions"
    );
    assert!(
        !wheel_writes.contains("\x1b[1;10r")
            && !wheel_writes.contains("\x1b[1S")
            && !wheel_writes.contains("\x1b[1T"),
        "wheel shifts must not restore DECSTBM or CSI S/T"
    );

    assert_eq!(
        tui.terminal.viewport()[4],
        "L60",
        "the terminal row used for the click must match the projected viewport"
    );

    let mouse = |kind, column| {
        InputEvent::Mouse(MouseEvent {
            kind,
            column,
            row: 4,
            modifiers: KeyModifiers::NONE,
        })
    };
    let _ = tui.dispatch_event(mouse(MouseEventKind::Down(MouseButton::Left), 0));
    let _ = tui.dispatch_event(mouse(MouseEventKind::Drag(MouseButton::Left), 3));
    tui.render_frame().expect("selection highlight");

    assert!(
        tui.terminal.viewport_cell(4, 0).reverse,
        "selection highlight must land on the clicked terminal row"
    );

    tui.terminal.clear_writes();
    let _ = tui.dispatch_event(mouse(MouseEventKind::Up(MouseButton::Left), 3));
    tui.render_frame().expect("selection release");
    assert!(
        tui.terminal.all_writes().contains("\x1b]52;c;TDYw\x07"),
        "the clicked visible row L60 must be the copied content"
    );
}

#[test]
fn application_owned_wheel_reprojects_without_component_render() {
    let lines: Vec<String> = (0..80).map(|i| format!("L{i:02}")).collect();
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 12),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    tui.add_child(Box::new(StaticLines { lines }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.render_frame().expect("warm");
    assert!(!tui.last_render_perf().ao_reprojected);

    tui.clear_ao_reproject_frames_for_test();
    let reaction = tui.dispatch_event(InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 2,
        row: 2,
        modifiers: KeyModifiers::NONE,
    }));
    assert_eq!(reaction, xylitol_tui::InputReaction::Rerender);
    tui.render_frame().expect("wheel");
    assert!(
        tui.last_render_perf().ao_reprojected,
        "wheel frame must skip Component::render"
    );
    assert_eq!(tui.ao_reproject_frames_for_test(), 1);
}

struct ViewportMarkLines {
    lines: Vec<String>,
}

impl Component for ViewportMarkLines {
    fn render(&mut self, _width: usize) -> Vec<String> {
        self.lines.clone()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
    fn dock_rows_hint(&self) -> Option<usize> {
        Some(1)
    }
    fn overlay_viewport(&self, lines: &mut [String], _width: usize, transcript_rows: usize) {
        let cap = transcript_rows.min(lines.len());
        if cap == 0 {
            return;
        }
        lines[0] = format!("{} VP-MARK", lines[0]);
    }
}

#[test]
fn overlay_viewport_pins_after_reproject() {
    let mut body: Vec<String> = (0..80).map(|i| format!("L{i:02}")).collect();
    body.push("dock".into());
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 12),
        InteractionMode::ApplicationOwned,
    );
    tui.add_child(Box::new(ViewportMarkLines { lines: body }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.render_frame().expect("ingest");
    assert!(!tui.last_render_perf().ao_reprojected);

    let notch = tui.application_owned_wheel_notch();
    assert!(tui.application_owned_scroll_by(-(notch * 4)));
    tui.clear_ao_reproject_frames_for_test();
    tui.render_frame().expect("reproject");
    assert!(tui.last_render_perf().ao_reprojected);
    assert!(tui.application_owned_scroll_top() > 0);

    let painted = tui.last_painted_lines();
    assert!(
        painted[0].contains("VP-MARK"),
        "visible top must carry overlay_viewport mark: {:?}",
        painted[0]
    );
    let content0 = tui
        .application_owned_content_lines()
        .and_then(|c| c.first())
        .cloned()
        .unwrap_or_default();
    assert!(
        !content0.contains("VP-MARK"),
        "ScrollView content[0] must stay unmarked: {content0:?}"
    );
}

/// Soft resize must not reproject width-specific cached lines (editor borders /
/// markdown wrap). Host uses `request_render(false)` on Resize.
#[test]
fn application_owned_soft_resize_rerenders_not_reprojects() {
    struct WidthFill;
    impl Component for WidthFill {
        fn render(&mut self, width: usize) -> Vec<String> {
            let body = "x".repeat(width.max(1));
            vec![body, "dock".into()]
        }
        fn handle_input(&mut self, _event: InputEvent) {}
        fn invalidate(&mut self) {}
        fn dock_rows_hint(&self) -> Option<usize> {
            Some(1)
        }
    }

    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.add_child(Box::new(WidthFill));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.render_frame().expect("warm 40");
    assert!(!tui.last_render_perf().ao_reprojected);

    tui.terminal.set_size_hint(20, 8);
    tui.request_render(false);
    tui.render_frame().expect("soft resize to 20");
    assert!(
        !tui.last_render_perf().ao_reprojected,
        "width change must rebuild components, not reproject 40-col cache"
    );
}

fn ao_seeded(
    term_rows: u16,
    content: Vec<String>,
    dock: Vec<String>,
) -> TUI<LoggingVirtualTerminal> {
    let mut lines = content;
    lines.extend(dock);
    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, term_rows),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    tui.add_child(Box::new(StaticLines { lines }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.request_render(true);
    tui.render_now().expect("seed");
    tui
}

fn mouse_at(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn hit_id_follows_live_scroll_not_registration_snapshot() {
    let content: Vec<String> = (0..20).map(|i| format!("row-{i:02}")).collect();
    let mut tui = ao_seeded(10, content, vec!["dock-a".into(), "dock-b".into()]);
    tui.mark_hit(Hit::on_line(7, 12).cols(0, 8));
    // 20 content rows, viewport 8 → follow-end scroll_top = 12.
    assert_eq!(tui.application_owned_scroll_top(), 12);
    assert_eq!(
        tui.hit_id_at(1, 0),
        Some(7),
        "screen 0 + scroll 12 → content 12"
    );
    assert!(tui.application_owned_scroll_by(-3));
    assert_eq!(tui.application_owned_scroll_top(), 9);
    assert_eq!(
        tui.hit_id_at(1, 0),
        None,
        "same screen cell must not keep the registration-time mapping"
    );
    assert_eq!(
        tui.hit_id_at(1, 3),
        Some(7),
        "screen 3 + scroll 9 → content 12"
    );
}

#[test]
fn viewport_hit_stays_on_screen_cell_after_scroll() {
    let content: Vec<String> = (0..20).map(|i| format!("row-{i:02}")).collect();
    let mut tui = ao_seeded(10, content, vec!["dock-a".into(), "dock-b".into()]);
    tui.mark_hit(Hit::at_cell(11, 1, 0));
    assert_eq!(tui.application_owned_scroll_top(), 12);
    assert_eq!(tui.hit_id_at(1, 0), Some(11));
    assert!(tui.application_owned_scroll_by(-3));
    assert_eq!(tui.application_owned_scroll_top(), 9);
    assert_eq!(
        tui.hit_id_at(1, 0),
        Some(11),
        "viewport-sticky hit must not drift with scroll"
    );
    assert_eq!(tui.hit_id_at(1, 3), None);
}

#[test]
fn default_down_swallow_does_not_start_drag_or_report_on_up() {
    let mut tui = ao_seeded(
        10,
        vec!["hello".into(), "world".into()],
        vec!["dock-a".into(), "dock-b".into()],
    );
    tui.mark_hit(Hit::on_line(3, 0).cols(0, 5));
    assert_eq!(tui.pointer_click_phase(), PointerClickPhase::DownSwallow);
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(mouse_at(
            MouseEventKind::Down(MouseButton::Left),
            1,
            0
        ))),
        xylitol_tui::InputReaction::Rerender
    );
    assert_eq!(tui.take_pointer_click(), Some(3));
    assert!(!tui.application_owned_is_dragging());
    assert!(!tui.application_owned_has_selection());
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Up(MouseButton::Left),
        1,
        0,
    )));
    assert!(tui.take_pointer_click().is_none());
}

#[test]
fn up_if_empty_reports_click_without_osc52() {
    let mut tui = ao_seeded(
        10,
        vec!["hello".into(), "world".into()],
        vec!["dock-a".into(), "dock-b".into()],
    );
    tui.mark_hit(Hit::on_line(9, 0).cols(0, 5));
    tui.set_pointer_click_phase(PointerClickPhase::UpIfEmptySelection);
    tui.terminal.clear_writes();
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Down(MouseButton::Left),
        1,
        0,
    )));
    assert!(tui.take_pointer_click().is_none());
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Up(MouseButton::Left),
        1,
        0,
    )));
    assert_eq!(tui.take_pointer_click(), Some(9));
    tui.request_render(false);
    let _ = tui.render_now();
    let raw = tui.terminal.all_writes();
    assert!(
        !raw.contains("\x1b]52;c;"),
        "empty click must not OSC52, got {raw:?}"
    );
}

#[test]
fn unarmed_moved_does_not_force_full_frame() {
    let mut tui = ao_seeded(
        10,
        vec!["hello".into(), "world".into()],
        vec!["dock-a".into(), "dock-b".into()],
    );
    let before = tui.frame_count();
    let stale_before = tui.ao_components_stale();
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(mouse_at(MouseEventKind::Moved, 1, 0))),
        xylitol_tui::InputReaction::None
    );
    tui.request_render(false);
    let _ = tui.try_render();
    assert_eq!(tui.frame_count(), before);
    assert_eq!(tui.ao_components_stale(), stale_before);
}

#[test]
fn armed_probe_same_id_moved_does_not_rerender() {
    let mut tui = ao_seeded(
        10,
        vec!["hello".into(), "world".into()],
        vec!["dock-a".into(), "dock-b".into()],
    );
    tui.mark_hit(Hit::on_line(1, 0).cols(0, 5));
    tui.arm_pointer_probe(true);
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(mouse_at(MouseEventKind::Moved, 1, 0))),
        xylitol_tui::InputReaction::Rerender
    );
    assert!(
        !tui.ao_components_stale(),
        "hover must not stale components"
    );
    assert_eq!(tui.pointer_hover_id(), Some(1));
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(mouse_at(MouseEventKind::Moved, 2, 0))),
        xylitol_tui::InputReaction::None,
        "same hit id must not schedule a frame"
    );
}

#[test]
fn hover_id_change_rerenders_without_stale_components() {
    let mut tui = ao_seeded(
        10,
        vec!["hello".into(), "world".into()],
        vec!["dock-a".into(), "dock-b".into()],
    );
    tui.replace_hits([Hit::on_line(1, 0).cols(0, 2), Hit::on_line(2, 0).cols(3, 5)]);
    tui.arm_pointer_probe(true);
    tui.mark_ao_components_stale();
    tui.render_frame().expect("warm");
    assert!(!tui.ao_components_stale());
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(MouseEventKind::Moved, 0, 0)));
    assert_eq!(tui.pointer_hover_id(), Some(1));
    assert!(!tui.ao_components_stale());
    assert_eq!(
        tui.dispatch_event(InputEvent::Mouse(mouse_at(MouseEventKind::Moved, 4, 0))),
        xylitol_tui::InputReaction::Rerender
    );
    assert_eq!(tui.pointer_hover_id(), Some(2));
    assert!(
        !tui.ao_components_stale(),
        "hover id change must reproject, not rebuild components"
    );
}

#[test]
fn capturing_region_blocks_transcript_drag() {
    let mut tui = ao_seeded(
        10,
        vec!["hello".into(), "world".into()],
        vec!["dock-a".into(), "dock-b".into()],
    );
    tui.mark_hit(Hit::on_line(4, 0).cols(0, 6).capturing());
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Down(MouseButton::Left),
        1,
        0,
    )));
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Drag(MouseButton::Left),
        5,
        0,
    )));
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Up(MouseButton::Left),
        5,
        0,
    )));
    assert!(!tui.application_owned_is_dragging());
    assert!(!tui.application_owned_has_selection());
}

// c2891 T2：gutter carve —— 组件按物理宽−1排版、stamp 后整行恰物理宽、
// wheel 快路径（previous_width==render_w）不被 carve 破坏。
#[test]
fn application_owned_gutter_carves_width_and_keeps_reproject_fast_path() {
    use xylitol_tui::{ScrollGutter, gutter_active};

    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    let body = "x".repeat(35);
    let lines: Vec<String> = (0..20).map(|i| format!("L{i:0>3}{body}")).collect();
    tui.add_child(Box::new(StaticLines { lines }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.set_scroll_gutter(Some(ScrollGutter));
    assert!(
        gutter_active(20, 6, 40),
        "content 18 > viewport 6, wide enough"
    );

    tui.request_render(true);
    tui.render_now().expect("first paint");
    // 顶行：39 列正文 + 末列 '^'；正文未被 stamp 侵入（col 38 仍是正文）
    assert_eq!(tui.terminal.inner().grid_row(0)[39].ch, '^');
    assert_eq!(tui.terminal.inner().grid_row(0)[38].ch, 'x');
    // dock 行不 stamp（grid 未写入 = '\0'）
    assert_eq!(tui.terminal.inner().grid_row(6)[39].ch, '\0');

    // wheel → reproject 快路径必须命中（carve 后 previous_width 一致性）
    tui.clear_ao_reproject_frames_for_test();
    assert!(tui.application_owned_scroll_by(-3));
    tui.request_render(false);
    tui.render_now().expect("wheel repaint");
    assert!(
        tui.ao_reproject_frames_for_test() >= 1,
        "carve must not break the reproject fast path"
    );
    assert_eq!(
        tui.terminal.inner().grid_row(0)[39].ch,
        '^',
        "still stamped"
    );
}

#[test]
fn application_owned_gutter_copy_excludes_gutter_column() {
    use xylitol_tui::ScrollGutter;

    let mut tui = TUI::with_interaction_mode(
        LoggingVirtualTerminal::new(40, 8),
        InteractionMode::ApplicationOwned,
    );
    tui.set_dock_rows(2);
    let body = "y".repeat(35);
    let lines: Vec<String> = (0..20).map(|i| format!("L{i:0>3}{body}")).collect();
    tui.add_child(Box::new(StaticLines { lines }));
    tui.terminal.start();
    tui.begin_application_owned_session();
    tui.set_scroll_gutter(Some(ScrollGutter));
    tui.request_render(true);
    tui.render_now().expect("paint");

    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Down(MouseButton::Left),
        0,
        0,
    )));
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Drag(MouseButton::Left),
        39,
        1,
    )));
    let _ = tui.dispatch_event(InputEvent::Mouse(mouse_at(
        MouseEventKind::Up(MouseButton::Left),
        39,
        1,
    )));
    assert!(tui.application_owned_has_selection());
    // copy-on-release 管线在 carve 下照常 armed（文本洁净由 runtime 单测锁）
    assert!(tui.copy_notice_active());
}
