mod support;

use std::sync::{Arc, Mutex};
use support::vt_feed::feed_vt;
use xylitol_tui::components::input::Input;
use xylitol_tui::tui::Component;

/// Helper: type characters and return the Input.
fn type_text(input: &mut Input, s: &str) {
    feed_vt(input, s);
}

#[test]
fn test_input_submits_value_with_backslash() {
    let mut input = Input::new();
    let value_set = Arc::new(Mutex::new(String::new()));
    let vs = value_set.clone();
    input.on_submit = Some(Box::new(move |v| {
        *vs.lock().unwrap() = v;
    }));

    type_text(&mut input, "hello\\");
    feed_vt(&mut input, "\r");
    assert_eq!(*value_set.lock().unwrap(), "hello\\");
}

#[test]
fn test_input_inserts_backslash_as_regular_char() {
    let mut input = Input::new();
    type_text(&mut input, "\\x");
    assert_eq!(input.value(), "\\x");
}

#[test]
fn test_input_cursor_moves_left() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    // Cursor at 5. Left twice = 3.
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "X");
    assert_eq!(input.value(), "helXlo");
}

#[test]
fn test_input_cursor_moves_right() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    // Left 3 = position 2, then right = 3
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "\x1b[C");
    feed_vt(&mut input, "X");
    assert_eq!(input.value(), "helXlo");
}

#[test]
fn test_input_cursor_home_end() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    feed_vt(&mut input, "\x1b[H"); // Home
    feed_vt(&mut input, "X");
    assert_eq!(input.value(), "Xhello");

    let mut input2 = Input::new();
    type_text(&mut input2, "hello");
    feed_vt(&mut input2, "\x1b[F"); // End
    feed_vt(&mut input2, "X");
    assert_eq!(input2.value(), "helloX");
}

#[test]
fn test_input_backspace() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    feed_vt(&mut input, "\x7f");
    assert_eq!(input.value(), "hell");
}

#[test]
fn test_input_delete_forward() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    // Left 2 = position 3
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "\x1b[D");
    feed_vt(&mut input, "\x1b[3~"); // Delete
    assert_eq!(input.value(), "helo");
}

#[test]
fn test_input_delete_word_backward() {
    let mut input = Input::new();
    type_text(&mut input, "foo bar baz");
    feed_vt(&mut input, "\x17"); // Ctrl+W
    assert_eq!(input.value(), "foo bar ");
}

#[test]
fn test_input_delete_to_line_start() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    // Ctrl+U from end: delete all to line start
    feed_vt(&mut input, "\x15");
    assert_eq!(input.value(), "");
}

#[test]
fn test_input_delete_to_line_end() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    feed_vt(&mut input, "\x1b[H"); // Home
    feed_vt(&mut input, "\x0b"); // Ctrl+K
    assert_eq!(input.value(), "");
}

#[test]
fn test_input_yank() {
    let mut input = Input::new();
    type_text(&mut input, "foo bar baz");
    feed_vt(&mut input, "\x17"); // Ctrl+W: kill "baz"
    assert_eq!(input.value(), "foo bar ");
    feed_vt(&mut input, "\x1b[H"); // Home
    feed_vt(&mut input, "\x19"); // Ctrl+Y: yank
    assert_eq!(input.value(), "bazfoo bar ");
}

#[test]
fn test_input_undo() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    feed_vt(&mut input, "X");
    feed_vt(&mut input, "Y");
    feed_vt(&mut input, "\x1f"); // Ctrl+_
    // Undo pops most recent push (helloXY state)
    // Should restore to either "hello" or "helloX"
    let val = input.value().to_string();
    assert!(!val.contains("Y"), "Undo should remove Y, got: {}", val);
}

#[test]
fn test_input_escape_callback() {
    let mut input = Input::new();
    let escaped = Arc::new(Mutex::new(false));
    let esc = escaped.clone();
    input.on_escape = Some(Box::new(move || {
        *esc.lock().unwrap() = true;
    }));
    feed_vt(&mut input, "\x1b");
    assert!(*escaped.lock().unwrap());
}

#[test]
fn test_input_render_cursor_visible() {
    let mut input = Input::new();
    input.set_focused(true);
    type_text(&mut input, "hello");
    let lines = input.render(20);
    assert!(lines[0].contains("\x1b_pi:c"));
    assert!(lines[0].contains("> "));
}

#[test]
fn test_input_render_cursor_not_visible_when_not_focused() {
    let mut input = Input::new();
    input.set_focused(false);
    type_text(&mut input, "hello");
    let lines = input.render(20);
    assert!(!lines[0].contains("\x1b_pi:c"));
}

#[test]
fn test_input_render_does_not_overflow() {
    let width = 80;
    let text = "hello world this is a test";
    let mut input = Input::new();
    input.set_focused(true);
    type_text(&mut input, text);
    let lines = input.render(width);
    let vis = xylitol_tui::utils::visible_width(&lines[0]);
    assert!(vis <= width, "Overflow: vis={} width={}", vis, width);
}
