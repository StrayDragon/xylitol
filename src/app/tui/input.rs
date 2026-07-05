//! Keyboard handling — crossterm events → [`InputOutcome`].
//!
//! MVP is single-line input: printable chars append, Backspace deletes, Enter
//! submits, Ctrl+C aborts. A leading `/` routes to slash parsing. A multiline
//! editor widget (e.g. tui-textarea) is intentionally deferred — see c340
//! design §6 "not doing".

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::tui::app::TuiApp;

/// The decision resulting from a key event.
pub enum InputOutcome {
    /// User pressed Enter on non-slash input: submit it as a prompt.
    Submit(String),
    /// User typed `/cmd`: route to slash command handling.
    Slash(String),
    /// Ctrl+C: abort the current turn (or quit if idle).
    Abort,
    /// Explicit quit (Ctrl+D).
    Quit,
    /// No actionable input (e.g. a bare modifier key, or still editing).
    Idle,
}

/// Map a crossterm key event to an outcome, mutating the app input buffer.
pub fn handle(event: KeyEvent, app: &mut TuiApp) -> InputOutcome {
    // Ctrl+C → abort, Ctrl+D → quit.
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        return match event.code {
            KeyCode::Char('c') => InputOutcome::Abort,
            KeyCode::Char('d') => InputOutcome::Quit,
            _ => InputOutcome::Idle,
        };
    }

    match event.code {
        KeyCode::Enter => {
            let text = app.input_buffer().trim().to_string();
            if text.is_empty() {
                return InputOutcome::Idle;
            }
            if let Some(stripped) = text.strip_prefix('/') {
                app.take_input();
                InputOutcome::Slash(stripped.to_string())
            } else {
                app.take_input();
                InputOutcome::Submit(text)
            }
        }
        KeyCode::Backspace => {
            app.backspace();
            InputOutcome::Idle
        }
        KeyCode::Left => {
            app.cursor_left();
            InputOutcome::Idle
        }
        KeyCode::Right => {
            app.cursor_right();
            InputOutcome::Idle
        }
        KeyCode::Home => {
            app.cursor_home();
            InputOutcome::Idle
        }
        KeyCode::End => {
            app.cursor_end();
            InputOutcome::Idle
        }
        KeyCode::Char(c) => {
            app.push_char(c);
            InputOutcome::Idle
        }
        // Other keys (modifiers alone, etc.) — ignored.
        _ => InputOutcome::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn submit_plain() {
        let mut app = TuiApp::default();
        for c in "hello".chars() {
            app.push_char(c);
        }
        match handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut app) {
            InputOutcome::Submit(s) => assert_eq!(s, "hello"),
            other => panic!("expected Submit, got {:?}", other_is(other)),
        }
        assert!(app.input_buffer().is_empty(), "submit clears the buffer");
    }

    #[test]
    fn slash_routes_to_slash() {
        let mut app = TuiApp::default();
        for c in "/model".chars() {
            app.push_char(c);
        }
        match handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut app) {
            InputOutcome::Slash(s) => assert_eq!(s, "model"),
            other => panic!("expected Slash, got {:?}", other_is(other)),
        }
    }

    #[test]
    fn ctrl_c_aborts() {
        let mut app = TuiApp::default();
        assert!(matches!(
            handle(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &mut app
            ),
            InputOutcome::Abort
        ));
    }

    #[test]
    fn backspace_pops() {
        let mut app = TuiApp::default();
        for c in "ab".chars() {
            app.push_char(c);
        }
        let _ = handle(
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
            &mut app,
        );
        assert_eq!(app.input_buffer(), "a");
    }

    #[test]
    fn empty_enter_is_idle() {
        let mut app = TuiApp::default();
        for c in "   ".chars() {
            app.push_char(c);
        }
        assert!(matches!(
            handle(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut app),
            InputOutcome::Idle
        ));
    }

    // Helper so panic! messages compile (InputOutcome has no Debug derive).
    fn other_is(_: InputOutcome) -> &'static str {
        "other"
    }

    // Keep `key` referenced for future char-by-char tests.
    #[test]
    fn char_appends() {
        let mut app = TuiApp::default();
        let _ = handle(key('x'), &mut app);
        assert_eq!(app.input_buffer(), "x");
    }
}
