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

    assert!(kb.matches_event(&key(KeyCode::Up), "tui.editor.cursorUp"));
    assert!(kb.matches_event(&key(KeyCode::Delete), "tui.editor.deleteCharForward"));
    assert!(kb.matches_event(&key(KeyCode::Esc), "tui.select.cancel"));
}

#[test]
fn test_keybindings_manager_get_keys() {
    let defs = create_default_definitions();
    let kb = KeybindingsManager::new(defs, HashMap::new());
    let keys = kb.get_keys("tui.editor.deleteWordBackward");
    assert!(keys.iter().any(|k| k == "ctrl+w"));
    assert!(keys.iter().any(|k| k == "alt+backspace"));
}

#[test]
fn test_keybindings_manager_custom_bindings() {
    let defs = create_default_definitions();
    let mut custom = HashMap::new();
    custom.insert("tui.input.submit".into(), vec!["ctrl+j".into()]);

    let kb = KeybindingsManager::new(defs, custom);
    assert!(!kb.matches_event(&key(KeyCode::Enter), "tui.input.submit"));
    assert!(kb.matches_event(
        &key_mod(KeyCode::Char('j'), KeyModifiers::CONTROL),
        "tui.input.submit"
    ));
}

#[test]
fn test_keybindings_manager_conflicts() {
    let defs = create_default_definitions();
    let mut custom = HashMap::new();
    custom.insert("tui.input.submit".into(), vec!["ctrl+x".into()]);
    custom.insert("tui.select.confirm".into(), vec!["ctrl+x".into()]);

    let kb = KeybindingsManager::new(defs, custom);
    let conflicts = kb.get_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].key, "ctrl+x");
}

#[test]
fn test_unknown_user_id_ignored() {
    let defs = create_default_definitions();
    let mut custom = HashMap::new();
    custom.insert("not.a.real.id".into(), vec!["f1".into()]);
    let kb = KeybindingsManager::new(defs, custom);
    assert!(kb.matches_event(&key(KeyCode::Enter), "tui.input.submit"));
}
