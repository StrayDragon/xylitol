mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::TuiTestHarness;
use xylitol_tui::components::input::Input;

#[test]
fn mutable_component_supports_cross_frame_mutation() {
    let lines = Rc::new(RefCell::new(vec!["hello".to_string()]));
    let mut h = TuiTestHarness::new(20, 3);
    h.mount_shared(lines.clone());
    h.render();
    h.assert_text_contains("hello");

    lines.borrow_mut().push("world".into());
    h.render();
    h.assert_text_contains("hello");
    h.assert_text_contains("world");
}

#[test]
fn keys_helper_drives_multi_step_sequence() {
    let mut input = Input::new();
    input.set_focused(true);

    let mut h = TuiTestHarness::new(20, 3);
    h.mount(Box::new(input));
    h.focus(Some(0));

    h.keys("abc");
    h.keys("\x1b[D");
    h.keys("\x7f");
    h.render();

    h.assert_text_contains("ac");
    let vp = h.tui.terminal.viewport().join("\n");
    let value_part = vp.trim_start_matches('>').trim_start();
    assert!(
        !value_part.starts_with("abc"),
        "backspace should have removed 'b', got: {vp:?}"
    );
    assert!(
        value_part.starts_with("ac"),
        "expected 'ac' after type-left-backspace, got: {value_part:?}"
    );
}

#[test]
fn harness_renders_same_instance_across_frames() {
    let lines = Rc::new(RefCell::new(vec!["first".to_string()]));
    let mut h = TuiTestHarness::new(20, 3);
    h.mount_shared(lines.clone());
    h.render();

    h.assert_text_contains("first");

    lines.borrow_mut()[0] = "second".into();
    h.tui.terminal.clear_writes();
    h.render();
    h.assert_text_contains("second");
    assert!(
        h.tui.terminal.write_count() > 0,
        "second frame on the same instance should write the diff"
    );
}

#[test]
fn container_concatenates_children_in_order() {
    use xylitol_tui::{Container, Text};

    let mut container = Container::new();
    container.add_child(Box::new(Text::new("alpha".into(), 0, 0)));
    container.add_child(Box::new(Text::new("beta".into(), 0, 0)));

    let mut h = TuiTestHarness::new(20, 5);
    h.mount(Box::new(container));
    h.render();

    let vp = h.tui.terminal.viewport().join("\n");
    let alpha = vp.find("alpha").expect("alpha present");
    let beta = vp.find("beta").expect("beta present");
    assert!(alpha < beta, "alpha must render before beta, got:\n{vp}");
}

#[test]
fn container_does_not_fanout_input() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use xylitol_tui::{Component, Container, InputEvent};

    let mut input = Input::new();
    input.set_focused(true);
    let mut container = Container::new();
    container.add_child(Box::new(input));

    container.handle_input(InputEvent::Key(KeyEvent {
        code: KeyCode::Char('x'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }));

    let lines = container.render(20);
    let joined = lines.join("\n");
    assert!(
        !joined.contains('x'),
        "Container must not fan out input; got: {joined:?}"
    );
}

#[test]
fn container_offer_pointer_fans_by_last_render_rows() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use std::cell::Cell;
    use std::rc::Rc;
    use xylitol_tui::{Component, Container, InputEvent, Spacer, Tabs};

    let changed = Rc::new(Cell::new(None));
    let mut tabs = Tabs::new(vec!["alpha".into(), "beta".into()]);
    tabs = tabs.with_on_change({
        let changed = changed.clone();
        move |i| changed.set(Some(i))
    });
    let mut container = Container::new();
    container.add_child(Box::new(Spacer::new(2)));
    container.add_child(Box::new(tabs));
    container.set_screen_origin(0, 0);
    let _ = container.render(40);

    let miss = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    assert!(!container.offer_pointer(&miss));
    assert_eq!(changed.get(), None);

    let hit = InputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 6,
        row: 2,
        modifiers: KeyModifiers::NONE,
    });
    assert!(container.offer_pointer(&hit));
    assert_eq!(changed.get(), Some(1));
}

#[test]
fn overlay_handle_hide_removes_from_composite() {
    use xylitol_tui::{OverlayAnchor, OverlayOptions, Text};

    let mut h = TuiTestHarness::new(20, 5);
    h.mount(Box::new(Text::new("base-line".into(), 0, 0)));
    let handle = h.tui.show_overlay(
        Box::new(Text::new("OVERLAY-MARK".into(), 0, 0)),
        OverlayOptions {
            anchor: Some(OverlayAnchor::TopLeft),
            margin: Some(xylitol_tui::OverlayMargin {
                top: Some(0),
                left: Some(0),
                right: None,
                bottom: None,
            }),
            ..Default::default()
        },
    );
    h.render();
    h.assert_text_contains("OVERLAY-MARK");

    handle.hide(&mut h.tui);
    h.render();
    let vp = h.tui.terminal.viewport().join("\n");
    assert!(
        !vp.contains("OVERLAY-MARK"),
        "hide must remove overlay from composite, got:\n{vp}"
    );
    h.assert_text_contains("base-line");
}

#[test]
fn overlay_handle_focus_brings_front() {
    use xylitol_tui::{OverlayAnchor, OverlayOptions, Text};

    let mut h = TuiTestHarness::new(20, 5);
    h.mount(Box::new(Text::new("base".into(), 0, 0)));
    let lower = h.tui.show_overlay(
        Box::new(Text::new("LOWER".into(), 0, 0)),
        OverlayOptions {
            anchor: Some(OverlayAnchor::TopLeft),
            margin: Some(xylitol_tui::OverlayMargin {
                top: Some(0),
                left: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    let upper = h.tui.show_overlay(
        Box::new(Text::new("UPPER".into(), 0, 0)),
        OverlayOptions {
            anchor: Some(OverlayAnchor::TopLeft),
            margin: Some(xylitol_tui::OverlayMargin {
                top: Some(0),
                left: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    assert!(upper.is_focused(&h.tui));
    assert!(!lower.is_focused(&h.tui));

    lower.focus(&mut h.tui);
    assert!(lower.is_focused(&h.tui));
    assert!(!upper.is_focused(&h.tui));
}
