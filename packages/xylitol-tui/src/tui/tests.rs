use super::*;
use crossterm::event::{KeyCode, KeyEventState, KeyModifiers};

struct DummyTerminal;

impl Terminal for DummyTerminal {
    fn write(&mut self, _data: &str) {}
    fn columns(&self) -> u16 {
        80
    }
    fn rows(&self) -> u16 {
        24
    }
    fn hide_cursor(&mut self) {}
    fn show_cursor(&mut self) {}
    fn clear_line(&mut self) {}
    fn clear_from_cursor(&mut self) {}
    fn clear_screen(&mut self) {}
    fn flush(&mut self) {}
}

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
fn dispatches_press_and_repeat() {
    let tui = TUI::new(DummyTerminal);

    assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Press)));
    assert!(tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Repeat)));
}

#[test]
fn ignores_release_without_opt_in() {
    let mut tui = TUI::new(DummyTerminal);
    tui.add_child(Box::new(DummyComponent {
        wants_release: false,
    }));
    tui.set_focus(Some(0));

    assert!(!tui.should_dispatch_key_event(&key_with_kind(KeyEventKind::Release)));
}

#[test]
fn dispatches_release_when_focused_component_requests_it() {
    let mut tui = TUI::new(DummyTerminal);
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
    let mut tui = TUI::new(DummyTerminal);
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
    let mut tui = TUI::new(DummyTerminal);
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
    let mut tui = TUI::new(DummyTerminal);
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
    let mut tui = TUI::new(DummyTerminal);
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
    let mut tui = TUI::new(DummyTerminal);
    tui.add_input_listener(|_| InputListenerResult::Consumed);
    assert_eq!(tui.dispatch_event(mouse_left_down()), InputReaction::None);
}

#[test]
fn mouse_listener_consumed_rerender_requests_frame() {
    let mut tui = TUI::new(DummyTerminal);
    tui.add_input_listener(|_| InputListenerResult::ConsumedRerender);
    assert_eq!(
        tui.dispatch_event(mouse_left_down()),
        InputReaction::Rerender
    );
}

#[test]
fn mouse_listener_consumed_blocks_focused_child() {
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tui = TUI::new(DummyTerminal);
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
    let mut tui = TUI::new(DummyTerminal);
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
    let mut tui = TUI::new(DummyTerminal);
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
fn mouse_consumed_listener_can_bump_frame_count() {
    let mut tui = TUI::new(DummyTerminal);
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
