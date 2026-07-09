use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use xylitol_tui::keybindings::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn key_mod(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, mods)
}

#[test]
fn test_default_keybindings_exist() {
    let defs = create_default_definitions();
    assert!(defs.contains_key("tui.editor.cursorUp"));
    assert!(defs.contains_key("tui.input.submit"));
    assert!(defs.contains_key("tui.select.cancel"));
}

#[test]
fn test_keybindings_manager_default() {
    let defs = create_default_definitions();
    let kb = KeybindingsManager::new(defs, HashMap::new());

    // Test cursorUp (default: "up")
    assert!(kb.matches_event(&key(KeyCode::Up), "tui.editor.cursorUp"));

    // Test deleteCharForward (default: ["delete", "ctrl+d"])
    assert!(kb.matches_event(&key(KeyCode::Delete), "tui.editor.deleteCharForward"));

    // Test cancel (default: ["escape", "ctrl+c"])
    assert!(kb.matches_event(&key(KeyCode::Esc), "tui.select.cancel"));
}

#[test]
fn test_keybindings_manager_get_keys() {
    let defs = create_default_definitions();
    let kb = KeybindingsManager::new(defs, HashMap::new());
    let keys = kb.get_keys("tui.editor.deleteWordBackward");
    assert!(keys.contains(&"ctrl+w"));
    assert!(keys.contains(&"alt+backspace"));
}

#[test]
fn test_keybindings_manager_custom_bindings() {
    let defs = create_default_definitions();
    let mut custom = HashMap::new();
    // Override submit to only match ctrl+j, removing default "enter"
    custom.insert("tui.input.submit", vec![("ctrl+j")]);

    let kb = KeybindingsManager::new(defs, custom);
    // Enter should no longer match submit (only ctrl+j should)
    assert!(!kb.matches_event(&key(KeyCode::Enter), "tui.input.submit"));
    // But ctrl+j should match
    assert!(kb.matches_event(
        &key_mod(KeyCode::Char('j'), KeyModifiers::CONTROL),
        "tui.input.submit"
    ));
}

#[test]
fn test_keybindings_manager_conflicts() {
    let defs = create_default_definitions();
    let mut custom = HashMap::new();
    // Bind the same key to two different actions
    custom.insert("tui.input.submit", vec![("ctrl+x")]);
    custom.insert("tui.select.confirm", vec![("ctrl+x")]);

    let kb = KeybindingsManager::new(defs, custom);
    let conflicts = kb.get_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].key, "ctrl+x");
}
