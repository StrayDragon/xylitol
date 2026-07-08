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
