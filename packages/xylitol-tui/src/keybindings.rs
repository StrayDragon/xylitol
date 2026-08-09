use crate::keys::matches_key_event;
use crossterm::event::KeyEvent;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

pub type Keybinding = &'static str;

#[derive(Debug, Clone)]
pub struct KeybindingDefinition {
    pub default_keys: Vec<&'static str>,
    pub description: Option<&'static str>,
}

pub type KeybindingDefinitions = HashMap<&'static str, KeybindingDefinition>;

/// User overrides from disk / tests (`id` → chords). Owned strings for JSON reload.
pub type KeybindingsConfig = HashMap<String, Vec<String>>;

#[derive(Debug, Clone)]
pub struct KeybindingConflict {
    pub key: String,
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
    // Unbound by default: bare ←→ reserved for product `/model` level cycle (c1470).
    // Page chords can be restored via keybindings.json when needed.
    ("tui.select.pageUp", &[], Some("Selection page up")),
    ("tui.select.pageDown", &[], Some("Selection page down")),
    ("tui.select.confirm", &["enter"], Some("Confirm selection")),
    (
        "tui.select.cancel",
        &["escape", "ctrl+c"],
        Some("Cancel selection"),
    ),
    (
        "tui.tree.foldOrUp",
        &["ctrl+left", "alt+left"],
        Some("Fold tree branch or jump up"),
    ),
    (
        "tui.tree.unfoldOrDown",
        &["ctrl+right", "alt+right"],
        Some("Unfold tree branch or jump down"),
    ),
    (
        "tui.tree.editLabel",
        &["shift+l"],
        Some("Edit tree annotation"),
    ),
    (
        "tui.tree.toggleLabelTimestamp",
        &["shift+t"],
        Some("Toggle annotation timestamps"),
    ),
];

pub struct KeybindingsManager {
    definitions: HashMap<&'static str, KeybindingDefinition>,
    keys_by_id: HashMap<&'static str, Vec<String>>,
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

        let mut user_claims: HashMap<String, Vec<&'static str>> = HashMap::new();
        for (kb, keys) in user_bindings.iter() {
            let Some((&id, _)) = self.definitions.iter().find(|(def_id, _)| *def_id == kb) else {
                continue; // unknown id — ignore
            };
            for key in keys {
                user_claims.entry(key.clone()).or_default().push(id);
            }
        }

        for (key, kbs) in &user_claims {
            if kbs.len() > 1 {
                self.conflicts.push(KeybindingConflict {
                    key: key.clone(),
                    keybindings: kbs.to_vec(),
                });
            }
        }

        for (&id, def) in self.definitions.iter() {
            let keys = if let Some(user_keys) = user_bindings.get(id) {
                user_keys.clone()
            } else {
                def.default_keys.iter().map(|s| (*s).to_string()).collect()
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

    pub fn get_keys(&self, keybinding: Keybinding) -> Vec<String> {
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

    /// Merge additional static definitions (e.g. product `app.*`) then rebuild with
    /// the given user overrides (or empty).
    pub fn extend_definitions(
        &mut self,
        extra: HashMap<&'static str, KeybindingDefinition>,
        user_bindings: KeybindingsConfig,
    ) {
        self.definitions.extend(extra);
        self.rebuild(&user_bindings);
    }
}

static GLOBAL_KEYBINDINGS: Mutex<Option<KeybindingsManager>> = Mutex::new(None);

thread_local! {
    /// HostSession / tests install a scoped manager so matching does not need the
    /// process-global mutex (and concurrent HostSession tests do not clobber each other).
    static SCOPED_KEYBINDINGS: RefCell<Option<Rc<RefCell<KeybindingsManager>>>> =
        const { RefCell::new(None) };
}

/// RAII install of a thread-local keybindings manager for [`with_keybindings`].
/// Restores the previous scoped value on drop (nested scopes supported).
pub struct KeybindingsScope {
    prev: Option<Rc<RefCell<KeybindingsManager>>>,
}

impl KeybindingsScope {
    pub fn enter(kb: Rc<RefCell<KeybindingsManager>>) -> Self {
        let prev = SCOPED_KEYBINDINGS.with(|slot| slot.borrow_mut().replace(kb));
        Self { prev }
    }
}

impl Drop for KeybindingsScope {
    fn drop(&mut self) {
        SCOPED_KEYBINDINGS.with(|slot| {
            *slot.borrow_mut() = self.prev.take();
        });
    }
}

pub fn set_keybindings(kb: KeybindingsManager) {
    *GLOBAL_KEYBINDINGS.lock().unwrap() = Some(kb);
}

pub fn with_keybindings<F, R>(f: F) -> R
where
    F: FnOnce(&KeybindingsManager) -> R,
{
    if let Some(kb) = SCOPED_KEYBINDINGS.with(|slot| slot.borrow().clone()) {
        return f(&kb.borrow());
    }
    let guard = GLOBAL_KEYBINDINGS.lock().unwrap();
    if let Some(ref kb) = *guard {
        f(kb)
    } else {
        drop(guard);
        let definitions = create_default_definitions();
        let mut guard = GLOBAL_KEYBINDINGS.lock().unwrap();
        *guard = Some(KeybindingsManager::new(definitions, HashMap::new()));
        f(guard.as_ref().unwrap())
    }
}

/// Mutate the scoped manager if present, else the process-global manager
/// (e.g. `set_user_bindings` on reload / package component tests).
pub fn with_keybindings_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut KeybindingsManager) -> R,
{
    if let Some(kb) = SCOPED_KEYBINDINGS.with(|slot| slot.borrow().clone()) {
        return f(&mut kb.borrow_mut());
    }
    let mut guard = GLOBAL_KEYBINDINGS.lock().unwrap();
    if guard.is_none() {
        let definitions = create_default_definitions();
        *guard = Some(KeybindingsManager::new(definitions, HashMap::new()));
    }
    f(guard.as_mut().unwrap())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn owned_override_replaces_default() {
        let defs = create_default_definitions();
        let mut custom = KeybindingsConfig::new();
        custom.insert("tui.input.submit".into(), vec!["ctrl+j".into()]);
        let kb = KeybindingsManager::new(defs, custom);
        assert!(!kb.matches_event(&key(KeyCode::Enter), "tui.input.submit"));
        assert!(kb.matches_event(
            &KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL),
            "tui.input.submit"
        ));
    }

    #[test]
    fn scoped_manager_preferred_for_matching() {
        use crossterm::event::{KeyCode, KeyModifiers};
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut custom = KeybindingsConfig::new();
        custom.insert("tui.input.submit".into(), vec!["ctrl+j".into()]);
        let scoped = Rc::new(RefCell::new(KeybindingsManager::new(
            create_default_definitions(),
            custom,
        )));
        let _guard = KeybindingsScope::enter(scoped);
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let ctrl_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
        assert!(!with_keybindings(
            |kb| kb.matches_event(&enter, "tui.input.submit")
        ));
        assert!(with_keybindings(
            |kb| kb.matches_event(&ctrl_j, "tui.input.submit")
        ));
    }
}
