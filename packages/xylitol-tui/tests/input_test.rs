use std::sync::{Arc, Mutex};
use xylitol_tui::components::input::Input;
use xylitol_tui::tui::Component;

/// Helper: type characters and return the Input.
fn type_text(input: &mut Input, s: &str) {
    for ch in s.chars() {
        input.handle_input(&ch.to_string());
    }
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
    input.handle_input("\r");
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
    input.handle_input("\x1b[D");
    input.handle_input("\x1b[D");
    input.handle_input("X");
    assert_eq!(input.value(), "helXlo");
}

#[test]
fn test_input_cursor_moves_right() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    // Left 3 = position 2, then right = 3
    input.handle_input("\x1b[D");
    input.handle_input("\x1b[D");
    input.handle_input("\x1b[D");
    input.handle_input("\x1b[C");
    input.handle_input("X");
    assert_eq!(input.value(), "helXlo");
}

#[test]
fn test_input_cursor_home_end() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    input.handle_input("\x1b[H"); // Home
    input.handle_input("X");
    assert_eq!(input.value(), "Xhello");

    let mut input2 = Input::new();
    type_text(&mut input2, "hello");
    input2.handle_input("\x1b[F"); // End
    input2.handle_input("X");
    assert_eq!(input2.value(), "helloX");
}

#[test]
fn test_input_backspace() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    input.handle_input("\x7f");
    assert_eq!(input.value(), "hell");
}

#[test]
fn test_input_delete_forward() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    // Left 2 = position 3
    input.handle_input("\x1b[D");
    input.handle_input("\x1b[D");
    input.handle_input("\x1b[3~"); // Delete
    assert_eq!(input.value(), "helo");
}

#[test]
fn test_input_delete_word_backward() {
    let mut input = Input::new();
    type_text(&mut input, "foo bar baz");
    input.handle_input("\x17"); // Ctrl+W
    assert_eq!(input.value(), "foo bar ");
}

#[test]
fn test_input_delete_to_line_start() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    // Ctrl+U from end: delete all to line start
    input.handle_input("\x15");
    assert_eq!(input.value(), "");
}

#[test]
fn test_input_delete_to_line_end() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    input.handle_input("\x1b[H"); // Home
    input.handle_input("\x0b"); // Ctrl+K
    assert_eq!(input.value(), "");
}

#[test]
fn test_input_yank() {
    let mut input = Input::new();
    type_text(&mut input, "foo bar baz");
    input.handle_input("\x17"); // Ctrl+W: kill "baz"
    assert_eq!(input.value(), "foo bar ");
    input.handle_input("\x1b[H"); // Home
    input.handle_input("\x19"); // Ctrl+Y: yank
    assert_eq!(input.value(), "bazfoo bar ");
}

#[test]
fn test_input_undo() {
    let mut input = Input::new();
    type_text(&mut input, "hello");
    input.handle_input("X");
    input.handle_input("Y");
    input.handle_input("\x1f"); // Ctrl+_
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
    input.handle_input("\x1b");
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
