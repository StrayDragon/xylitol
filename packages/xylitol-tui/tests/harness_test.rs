//! TuiTestHarness + MutableComponent tests (c405 layer 1, spec tt02).
//!
//! Validates the generalized key-sequence driver: mount a widget, feed a byte
//! string of terminal input through `keys()`, assert on the resulting state —
//! without rebuilding a second TUI instance (the workaround this replaces).

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::TuiTestHarness;
use xylitol_tui::components::input::Input;

/// The `MutableComponent` (generalized from virtual_terminal_test.rs) lets a
/// test mutate content between frames via a shared `Rc<RefCell<>>` handle,
/// so cross-frame diff scenarios no longer need a second TUI instance.
#[test]
fn mutable_component_supports_cross_frame_mutation() {
    let lines = Rc::new(RefCell::new(vec!["hello".to_string()]));
    let mut h = TuiTestHarness::new(20, 3);
    h.mount_shared(lines.clone());
    h.render();
    h.assert_text_contains("hello");

    // Mutate the shared state and re-render the SAME instance.
    lines.borrow_mut().push("world".into());
    h.render();
    h.assert_text_contains("hello");
    h.assert_text_contains("world");
}

/// Drive an Input through a multi-key sequence (type abc, Left, Backspace)
/// and verify the resulting text — the helix-style model assertion pattern.
/// Covers scenario tt02: typed abc, left-arrow moved cursor, backspace
/// deleted b, leaving "ab".
#[test]
fn keys_helper_drives_multi_step_sequence() {
    let mut input = Input::new();
    input.set_focused(true);

    let mut h = TuiTestHarness::new(20, 3);
    h.mount(Box::new(input));
    h.focus(Some(0));

    // Type "abc", move cursor left once (now between b and c), backspace
    // (deletes b) → remaining text is "ac".
    h.keys("abc");
    h.keys("\x1b[D"); // Left arrow (CSI D)
    h.keys("\x7f"); // Backspace
    h.render();

    // The first Input line renders "> " prefix + value. After the sequence
    // the value should be "ac" (b deleted). We assert via the rendered output
    // rather than reaching into the owned component (the harness owns it now).
    h.assert_text_contains("ac");
    // b was deleted so it must not appear.
    let vp = h.tui.terminal.viewport().join("\n");
    // 'b' may appear coincidentally elsewhere; check the typed region: after
    // "> " we expect "ac" not "abc". We strip the prompt prefix to verify.
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

/// The harness drives the same instance across two frames (no rebuild),
/// fixing the workaround documented at virtual_terminal_test.rs:178-184.
#[test]
fn harness_renders_same_instance_across_frames() {
    let lines = Rc::new(RefCell::new(vec!["first".to_string()]));
    let mut h = TuiTestHarness::new(20, 3);
    h.mount_shared(lines.clone());
    h.render();

    // First frame renders.
    h.assert_text_contains("first");

    // Change content and render again — same harness, same TUI instance.
    lines.borrow_mut()[0] = "second".into();
    h.tui.terminal.clear_writes();
    h.render();
    h.assert_text_contains("second");
    // A second frame with changed content must have written something
    // (the diff path ran on the same instance, not a rebuilt one).
    assert!(
        h.tui.terminal.write_count() > 0,
        "second frame on the same instance should write the diff"
    );
}

/// Container concatenates child render lines in order (c445 / pte01).
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

/// Container does not fan out InputEvent to children (c445 / pte01).
#[test]
fn container_does_not_fanout_input() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use xylitol_tui::{Component, Container, InputEvent};

    let mut input = Input::new();
    input.set_focused(true);
    let mut container = Container::new();
    container.add_child(Box::new(input));

    // Direct handle_input on Container must not reach the child.
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

/// OverlayHandle hide removes overlay from the composite (c445 / pte02).
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

/// OverlayHandle.focus brings a lower overlay to the front (c445 / pte03).
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
