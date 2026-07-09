use crate::keys::{KeyId, matches_key_event};
use crossterm::event::KeyEvent;
use std::collections::HashMap;
use std::sync::Mutex;

pub type Keybinding = &'static str;

#[derive(Debug, Clone)]
pub struct KeybindingDefinition {
    pub default_keys: Vec<KeyId>,
    pub description: Option<&'static str>,
}

pub type KeybindingDefinitions = HashMap<&'static str, KeybindingDefinition>;
pub type KeybindingsConfig = HashMap<&'static str, Vec<KeyId>>;

#[derive(Debug, Clone)]
pub struct KeybindingConflict {
    pub key: KeyId,
    pub keybindings: Vec<&'static str>,
}

static TUI_KEYBINDINGS: &[(&str, &[&str], Option<&str>)] = &[
    ("tui.editor.cursorUp", &["up"], Some("Move cursor up")),
    ("tui.editor.cursorDown", &["down"], Some("Move cursor down")),
    (
        "tui.editor.cursorLeft",
        &["left", "ctrl+b"],
        Some("Move cursor left"),
    ),
    (
        "tui.editor.cursorRight",
        &["right", "ctrl+f"],
        Some("Move cursor right"),
    ),
    (
        "tui.editor.cursorWordLeft",
        &["alt+left", "ctrl+left", "alt+b"],
        Some("Move cursor word left"),
    ),
    (
        "tui.editor.cursorWordRight",
        &["alt+right", "ctrl+right", "alt+f"],
        Some("Move cursor word right"),
    ),
    (
        "tui.editor.cursorLineStart",
        &["home", "ctrl+a"],
        Some("Move to line start"),
    ),
    (
        "tui.editor.cursorLineEnd",
        &["end", "ctrl+e"],
        Some("Move to line end"),
    ),
    (
        "tui.editor.jumpForward",
        &["ctrl+]"],
        Some("Jump forward to character"),
    ),
    (
        "tui.editor.jumpBackward",
        &["ctrl+alt+]"],
        Some("Jump backward to character"),
    ),
    ("tui.editor.pageUp", &["pageUp"], Some("Page up")),
    ("tui.editor.pageDown", &["pageDown"], Some("Page down")),
    (
        "tui.editor.deleteCharBackward",
        &["backspace"],
        Some("Delete character backward"),
    ),
    (
        "tui.editor.deleteCharForward",
        &["delete", "ctrl+d"],
        Some("Delete character forward"),
    ),
    (
        "tui.editor.deleteWordBackward",
        &["ctrl+w", "alt+backspace"],
        Some("Delete word backward"),
    ),
    (
        "tui.editor.deleteWordForward",
        &["alt+d", "alt+delete"],
        Some("Delete word forward"),
    ),
    (
        "tui.editor.deleteToLineStart",
        &["ctrl+u"],
        Some("Delete to line start"),
    ),
    (
        "tui.editor.deleteToLineEnd",
        &["ctrl+k"],
        Some("Delete to line end"),
    ),
    ("tui.editor.yank", &["ctrl+y"], Some("Yank")),
    ("tui.editor.yankPop", &["alt+y"], Some("Yank pop")),
    ("tui.editor.undo", &["ctrl+-"], Some("Undo")),
    (
        "tui.input.newLine",
        &["shift+enter", "ctrl+j"],
        Some("Insert newline"),
    ),
    ("tui.input.submit", &["enter"], Some("Submit input")),
    ("tui.input.tab", &["tab"], Some("Tab / autocomplete")),
    ("tui.input.copy", &["ctrl+c"], Some("Copy selection")),
    ("tui.select.up", &["up"], Some("Move selection up")),
    ("tui.select.down", &["down"], Some("Move selection down")),
    ("tui.select.pageUp", &["pageUp"], Some("Selection page up")),
    (
        "tui.select.pageDown",
        &["pageDown"],
        Some("Selection page down"),
    ),
    ("tui.select.confirm", &["enter"], Some("Confirm selection")),
    (
        "tui.select.cancel",
        &["escape", "ctrl+c"],
        Some("Cancel selection"),
    ),
];

pub struct KeybindingsManager {
    definitions: HashMap<&'static str, KeybindingDefinition>,
    keys_by_id: HashMap<&'static str, Vec<KeyId>>,
    conflicts: Vec<KeybindingConflict>,
}

impl KeybindingsManager {
    pub fn new(
        definitions: HashMap<&'static str, KeybindingDefinition>,
        user_bindings: KeybindingsConfig,
    ) -> Self {
        let mut mgr = Self {
            definitions,
            keys_by_id: HashMap::new(),
            conflicts: Vec::new(),
        };
        mgr.rebuild(&user_bindings);
        mgr
    }

    fn rebuild(&mut self, user_bindings: &KeybindingsConfig) {
        self.keys_by_id.clear();
        self.conflicts.clear();

        let mut user_claims: HashMap<KeyId, Vec<&str>> = HashMap::new();
        for (kb, keys) in user_bindings.iter() {
            if !self.definitions.contains_key(kb) {
                continue;
            }
            for key in keys {
                user_claims.entry(key).or_default().push(kb);
            }
        }

        for (key, kbs) in &user_claims {
            if kbs.len() > 1 {
                self.conflicts.push(KeybindingConflict {
                    key,
                    keybindings: kbs.to_vec(),
                });
            }
        }

        for (&id, def) in self.definitions.iter() {
            let keys = if let Some(user_keys) = user_bindings.get(id) {
                user_keys.clone()
            } else {
                def.default_keys.clone()
            };
            self.keys_by_id.insert(id, keys);
        }
    }

    pub fn matches_event(&self, event: &KeyEvent, keybinding: Keybinding) -> bool {
        if let Some(keys) = self.keys_by_id.get(keybinding) {
            for key in keys {
                if matches_key_event(event, key) {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_keys(&self, keybinding: Keybinding) -> Vec<KeyId> {
        self.keys_by_id.get(keybinding).cloned().unwrap_or_default()
    }

    pub fn get_definition(&self, keybinding: Keybinding) -> Option<&KeybindingDefinition> {
        self.definitions.get(keybinding)
    }

    pub fn get_conflicts(&self) -> &[KeybindingConflict] {
        &self.conflicts
    }

    pub fn set_user_bindings(&mut self, user_bindings: KeybindingsConfig) {
        self.rebuild(&user_bindings);
    }
}

static GLOBAL_KEYBINDINGS: Mutex<Option<KeybindingsManager>> = Mutex::new(None);

pub fn set_keybindings(kb: KeybindingsManager) {
    *GLOBAL_KEYBINDINGS.lock().unwrap() = Some(kb);
}

pub fn with_keybindings<F, R>(f: F) -> R
where
    F: FnOnce(&KeybindingsManager) -> R,
{
    let guard = GLOBAL_KEYBINDINGS.lock().unwrap();
    if let Some(ref kb) = *guard {
        f(kb)
    } else {
        // Lazy init with defaults
        drop(guard);
        let definitions = create_default_definitions();
        let mut guard = GLOBAL_KEYBINDINGS.lock().unwrap();
        *guard = Some(KeybindingsManager::new(definitions, HashMap::new()));
        f(guard.as_ref().unwrap())
    }
}

/// Create default keybinding definitions from TUI_KEYBINDINGS.
pub fn create_default_definitions() -> HashMap<&'static str, KeybindingDefinition> {
    let mut map = HashMap::new();
    for &(kb, keys, desc) in TUI_KEYBINDINGS.iter() {
        map.insert(
            kb,
            KeybindingDefinition {
                default_keys: keys.to_vec(),
                description: desc,
            },
        );
    }
    map
}
