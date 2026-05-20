use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::component::Component;
use super::event::{AppAction, TuiEvent};
use super::input::InputComponent;
use super::slash::Completer;

#[test]
fn tab_submits_non_bang_draft_as_queue_or_submit() {
    let mut input = InputComponent::new(Completer::new());
    input.load_text("hello");

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    let result = input.handle_event(&TuiEvent::Key(key));

    assert!(result.consumed);
    let Some(action) = result.action else {
        panic!("expected QueueOrSubmit action");
    };
    match action {
        AppAction::QueueOrSubmit(text) => assert_eq!(text, "hello"),
        other => panic!("unexpected action: {other:?}"),
    }
    assert!(input.is_empty());
}

#[test]
fn tab_does_not_submit_bang_shell_draft() {
    let mut input = InputComponent::new(Completer::new());
    input.load_text("!ls");

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    let result = input.handle_event(&TuiEvent::Key(key));

    assert!(result.consumed);
    assert!(result.action.is_none());
    assert_eq!(input.text(), "!ls".to_string());
}

#[test]
fn esc_cancels_draft() {
    let mut input = InputComponent::new(Completer::new());
    input.load_text("hello");

    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let result = input.handle_event(&TuiEvent::Key(key));

    assert!(result.consumed);
    assert!(input.is_empty());
}

#[test]
fn esc_dismisses_slash_popup_without_clearing_input() {
    let mut input = InputComponent::new(Completer::new());
    input.load_text("/cl");
    assert!(input.slash_popup_active());

    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let result = input.handle_event(&TuiEvent::Key(key));

    assert!(result.consumed);
    assert!(!input.slash_popup_active());
    // After dismissal, input should still be present; Esc should require a second press to clear.
    assert_eq!(input.text(), "/cl".to_string());
}

#[test]
fn esc_on_empty_composer_primes_backtrack() {
    let mut input = InputComponent::new(Completer::new());
    assert!(input.is_empty());

    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let result = input.handle_event(&TuiEvent::Key(key));

    assert!(result.consumed);
    match result.action {
        Some(AppAction::BacktrackPrime) => {}
        other => panic!("unexpected action: {other:?}"),
    }
}
