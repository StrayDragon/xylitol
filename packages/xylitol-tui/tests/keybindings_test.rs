use std::collections::HashMap;
use xylitol_tui::keybindings::*;

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
    assert!(kb.matches("\x1b[A", "tui.editor.cursorUp"));

    // Test deleteCharForward (default: ["delete", "ctrl+d"])
    assert!(kb.matches("\x1b[3~", "tui.editor.deleteCharForward"));

    // Test cancel (default: ["escape", "ctrl+c"])
    assert!(kb.matches("\x1b", "tui.select.cancel"));
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
    assert!(!kb.matches("\r", "tui.input.submit"));
    // But ctrl+j should match
    assert!(kb.matches("\n", "tui.input.submit"));
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
