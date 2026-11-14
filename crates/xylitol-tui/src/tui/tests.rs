use super::*;
use crossterm::event::{KeyCode, KeyEventState, KeyModifiers};

use crate::NullTerminal;

struct DummyComponent {
    wants_release: bool,
}

impl Component for DummyComponent {
    fn render(&mut self, _width: usize) -> Vec<String> {
        Vec::new()
    }

    fn handle_input(&mut self, _event: InputEvent) {}

    fn invalidate(&mut self) {}

    fn wants_key_release(&self) -> bool {
        self.wants_release
    }
}

fn key_with_kind(kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::NONE,
        kind,
        state: KeyEventState::NONE,
    }
}

#[test]
fn pointer_view_uses_declared_dock_until_something_is_painted() {
    let mut tui = TUI::new(NullTerminal::default());
    tui.set_dock_rows(4);
    let view = tui.pointer_view(false);
    assert_eq!(
        view.transcript_rows,
        tui.terminal.rows().saturating_sub(4),
        "with no painted dock the view falls back to the declared rows"
    );
}

#[test]
fn dispatches_press_and_repeat() {
    let tui = TUI::new(NullTerminal::default());

    assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Press)));
    assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Repeat)));
}

#[test]
fn ignores_release_without_opt_in() {
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(DummyComponent {
        wants_release: false,
    }));
    tui.set_focus(Some(0));

    assert!(!tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Release)));
}

#[test]
fn dispatches_release_when_focused_component_requests_it() {
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(DummyComponent {
        wants_release: true,
    }));
    tui.set_focus(Some(0));

    assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Release)));
}

struct RecordingComponent {
    events: std::sync::Arc<std::sync::Mutex<Vec<InputEvent>>>,
}

impl Component for RecordingComponent {
    fn render(&mut self, _width: usize) -> Vec<String> {
        Vec::new()
    }
    fn handle_input(&mut self, event: InputEvent) {
        self.events.lock().unwrap().push(event);
    }
    fn invalidate(&mut self) {}
}

fn letter_a() -> InputEvent {
    InputEvent::Key(KeyEvent {
        code: KeyCode::Char('a'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn ctrl_c() -> InputEvent {
    InputEvent::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn input_listener_consumes_before_focus() {
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(RecordingComponent {
        events: events.clone(),
    }));
    tui.set_focus(Some(0));
    tui.add_input_listener(|ev| {
        if matches!(
            &ev,
            InputEvent::Key(k)
                if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL)
        ) {
            InputListenerResult::Consumed
        } else {
            InputListenerResult::Continue
        }
    });

    tui.dispatch_event(ctrl_c());
    assert!(events.lock().unwrap().is_empty());

    tui.dispatch_event(letter_a());
    assert_eq!(events.lock().unwrap().len(), 1);
}

#[test]
fn input_listeners_run_fifo_until_consumed() {
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tui = TUI::new(NullTerminal::default());
    let o1 = order.clone();
    tui.add_input_listener(move |_| {
        o1.lock().unwrap().push(1);
        InputListenerResult::Continue
    });
    let o2 = order.clone();
    tui.add_input_listener(move |_| {
        o2.lock().unwrap().push(2);
        InputListenerResult::Consumed
    });
    let o3 = order.clone();
    tui.add_input_listener(move |_| {
        o3.lock().unwrap().push(3);
        InputListenerResult::Continue
    });

    tui.dispatch_event(letter_a());
    assert_eq!(*order.lock().unwrap(), vec![1, 2]);
}

#[test]
fn paste_passes_through_continuing_listener() {
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(RecordingComponent {
        events: events.clone(),
    }));
    tui.set_focus(Some(0));
    tui.add_input_listener(|_| InputListenerResult::Continue);

    tui.dispatch_event(InputEvent::Paste("hello".into()));
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[InputEvent::Paste("hello".into())]
    );
}

fn mouse_moved() -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 1,
        row: 2,
        modifiers: KeyModifiers::NONE,
    })
}

fn mouse_left_down() -> InputEvent {
    InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 3,
        row: 4,
        modifiers: KeyModifiers::NONE,
    })
}

#[test]
fn mouse_motion_is_pointer_motion() {
    assert!(mouse_moved().is_pointer_motion());
    assert!(!mouse_left_down().is_pointer_motion());
    assert!(!letter_a().is_pointer_motion());
}

#[test]
fn mouse_down_reaches_focused_child_without_default_rerender() {
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(RecordingComponent {
        events: events.clone(),
    }));
    tui.set_focus(Some(0));

    let reaction = tui.dispatch_event(mouse_left_down());
    assert_eq!(reaction, InputReaction::None);
    assert_eq!(events.lock().unwrap().len(), 1);
    assert!(matches!(
        events.lock().unwrap()[0],
        InputEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(_),
            ..
        })
    ));
}

#[test]
fn mouse_listener_consumed_does_not_request_rerender() {
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_input_listener(|_| InputListenerResult::Consumed);
    assert_eq!(tui.dispatch_event(mouse_left_down()), InputReaction::None);
}

#[test]
fn mouse_listener_consumed_rerender_requests_frame() {
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_input_listener(|_| InputListenerResult::ConsumedRerender);
    assert_eq!(
        tui.dispatch_event(mouse_left_down()),
        InputReaction::Rerender
    );
}

#[test]
fn mouse_listener_consumed_blocks_focused_child() {
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(RecordingComponent {
        events: events.clone(),
    }));
    tui.set_focus(Some(0));
    tui.add_input_listener(|_| InputListenerResult::Consumed);
    assert_eq!(tui.dispatch_event(mouse_left_down()), InputReaction::None);
    assert!(
        events.lock().unwrap().is_empty(),
        "Consumed listener must block focused child"
    );
}

struct MouseAwareComponent;

impl Component for MouseAwareComponent {
    fn render(&mut self, _width: usize) -> Vec<String> {
        Vec::new()
    }
    fn handle_input(&mut self, _event: InputEvent) {}
    fn invalidate(&mut self) {}
    fn input_wants_rerender(&self, event: &InputEvent) -> bool {
        matches!(
            event,
            InputEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(_),
                ..
            })
        )
    }
}

#[test]
fn component_can_opt_in_mouse_rerender() {
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(MouseAwareComponent));
    tui.set_focus(Some(0));
    assert_eq!(
        tui.dispatch_event(mouse_left_down()),
        InputReaction::Rerender
    );
    assert_eq!(tui.dispatch_event(mouse_moved()), InputReaction::None);
}

#[test]
fn mouse_moved_flood_does_not_bump_frame_count() {
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(RecordingComponent {
        events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
    }));
    tui.set_focus(Some(0));
    let before = tui.frame_count();
    for _ in 0..64 {
        if tui.dispatch_event(mouse_moved()) == InputReaction::Rerender {
            tui.request_render(false);
        }
    }
    let _ = tui.try_render();
    assert_eq!(
        tui.frame_count(),
        before,
        "Moved flood must not schedule or paint frames"
    );
}

#[test]
fn offer_pointer_on_root_tabs_blocks_focused_child() {
    use crate::components::tabs::Tabs;

    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tabs = Tabs::new(vec!["alpha".into(), "beta".into(), "gamma".into()]);
    let _ = tabs.render(80);
    let (_, start, end) = tabs.title_col_ranges()[1];
    let col = ((start + end) / 2) as u16;

    let mut tui = TUI::new(NullTerminal::default());
    tui.add_child(Box::new(tabs));
    tui.add_child(Box::new(RecordingComponent {
        events: events.clone(),
    }));
    tui.set_focus(Some(1));

    let click = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: col,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    tui.dispatch_event(click);
    assert!(
        events.lock().unwrap().is_empty(),
        "focused child must not receive a pointer Tabs claimed"
    );
}

#[test]
fn mouse_consumed_listener_can_bump_frame_count() {
    let mut tui = TUI::new(NullTerminal::default());
    tui.add_input_listener(|_| InputListenerResult::ConsumedRerender);
    let before = tui.frame_count();
    assert_eq!(
        tui.dispatch_event(mouse_left_down()),
        InputReaction::Rerender
    );
    tui.request_render(false);
    tui.try_render().expect("ConsumedRerender mouse may paint");
    assert!(
        tui.frame_count() > before,
        "ConsumedRerender mouse Down may schedule a frame"
    );
}

#[test]
fn overlay_clip_intersects_and_records_rect() {
    use crate::components::text::Text;
    use crate::selection::ScreenRect;
    use crate::tui::{OverlayAnchor, OverlayOptions, SizeValue};

    struct Lines(usize);
    impl Component for Lines {
        fn render(&mut self, _width: usize) -> Vec<String> {
            (0..self.0).map(|i| format!("line{i}")).collect()
        }
        fn handle_input(&mut self, _event: InputEvent) {}
        fn invalidate(&mut self) {}
    }

    let mut tui = TUI::new(NullTerminal::new(40, 20));
    tui.add_child(Box::new(Lines(20)));
    let clip = ScreenRect {
        row: 2,
        col: 5,
        height: 4,
        width: 10,
    };
    tui.show_overlay(
        Box::new(Text::new("OVERLAY".into(), 0, 0)),
        OverlayOptions {
            width: Some(SizeValue::Absolute(10)),
            max_width: Some(10),
            max_height: Some(SizeValue::Absolute(5)),
            min_height: Some(1),
            anchor: Some(OverlayAnchor::TopRight),
            clip: Some(clip),
            ..Default::default()
        },
    );
    tui.render_frame().expect("render");
    let rect = tui.overlays[0].2.last_screen_rect.expect("rect");
    assert!(rect.row >= clip.row);
    assert!(rect.col >= clip.col);
    assert!(rect.row + rect.height <= clip.row + clip.height);
    assert!(rect.col + rect.width <= clip.col + clip.width);
    let mut found = false;
    for row in clip.row as usize..(clip.row as usize + clip.height as usize) {
        if tui
            .previous_lines
            .get(row)
            .is_some_and(|line| line.contains("OVERLAY"))
        {
            found = true;
        }
    }
    assert!(found, "overlay must be composited inside clip");
    for line in tui.previous_lines.iter().take(clip.row as usize) {
        assert!(!line.contains("OVERLAY"));
    }
}
