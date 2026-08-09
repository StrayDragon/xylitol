//! Product keybinding catalog + disk hot-reload (c1090).
//!
//! Merges `app.*` defaults into the package [`KeybindingsManager`], loads
//! `keybindings.json` from the agent dir, and exposes [`reload_keybindings`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use xylitol_tui::{
    KeybindingDefinition, KeybindingsConfig, KeybindingsManager, create_default_definitions,
    set_keybindings, with_keybindings, with_keybindings_mut,
};

/// Product action defaults (pi-aligned ids; chords match previous hardcoding).
static APP_KEYBINDINGS: &[(&str, &[&str], Option<&str>)] = &[
    ("app.interrupt", &["escape"], Some("Cancel or abort")),
    (
        "app.clear",
        &["ctrl+c"],
        Some("Clear editor / exit when empty"),
    ),
    (
        "app.message.followUp",
        &["alt+enter"],
        Some("Queue follow-up message"),
    ),
    (
        "app.message.dequeue",
        &["alt+up"],
        Some("Restore queued messages"),
    ),
    (
        "app.editor.external",
        &["ctrl+g"],
        Some("Open external editor"),
    ),
    (
        "app.thinking.toggle",
        &["ctrl+t"],
        Some("Toggle thinking blocks"),
    ),
    (
        "app.tools.expand",
        &["ctrl+o"],
        Some("Toggle tool output viewport"),
    ),
    (
        "app.tools.blocks",
        &["alt+e"],
        Some("Toggle tool/diff block expand"),
    ),
    (
        "app.tree.filter.default",
        &["ctrl+d"],
        Some("Tree filter: default"),
    ),
    (
        "app.tree.filter.noTools",
        &["ctrl+t"],
        Some("Tree filter: hide tools"),
    ),
    (
        "app.tree.filter.userOnly",
        &["ctrl+u"],
        Some("Tree filter: user only"),
    ),
    (
        "app.tree.filter.labeledOnly",
        &["ctrl+l"],
        Some("Tree filter: labeled only"),
    ),
    ("app.tree.filter.all", &["ctrl+a"], Some("Tree filter: all")),
    (
        "app.tree.filter.cycleForward",
        &["ctrl+o"],
        Some("Tree filter: cycle forward"),
    ),
    (
        "app.tree.filter.cycleBackward",
        &["ctrl+shift+o"],
        Some("Tree filter: cycle backward"),
    ),
    (
        "app.session.fork",
        &["shift+f"],
        Some("Fork session at node"),
    ),
    ("app.tree.editLabel", &["shift+l"], Some("Edit tree label")),
    (
        "app.tree.toggleLabelTimestamp",
        &["shift+t"],
        Some("Toggle label timestamps"),
    ),
    (
        "app.session.toggleSort",
        &["ctrl+s"],
        Some("Resume: cycle sort"),
    ),
    (
        "app.session.toggleNamedFilter",
        &["ctrl+n"],
        Some("Resume: named filter"),
    ),
    (
        "app.session.togglePath",
        &["ctrl+p"],
        Some("Resume: toggle path"),
    ),
    (
        "app.session.toggleId",
        &["ctrl+u"],
        Some("Resume: toggle session id"),
    ),
    ("app.session.rename", &["ctrl+r"], Some("Resume: rename")),
    ("app.session.delete", &["ctrl+d"], Some("Resume: delete")),
    (
        "app.paste.image",
        &["ctrl+v"],
        Some("Paste clipboard image as tempfile path"),
    ),
];

/// Result of a reload attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadOutcome {
    /// File missing — defaults kept / restored from empty override.
    NoFile {
        path: PathBuf,
    },
    Applied {
        path: PathBuf,
    },
    Failed {
        path: PathBuf,
        error: String,
    },
}

fn app_definitions() -> HashMap<&'static str, KeybindingDefinition> {
    let mut map = HashMap::new();
    for &(id, keys, desc) in APP_KEYBINDINGS {
        map.insert(
            id,
            KeybindingDefinition {
                default_keys: keys.to_vec(),
                description: desc,
            },
        );
    }
    map
}

fn keybindings_path(agent_dir: &Path) -> PathBuf {
    agent_dir.join("keybindings.json")
}

/// Default agent directory (`~/.xylitol/`) without reaching `infra`.
pub fn default_agent_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xylitol")
}

/// Parse `keybindings.json` object: `{ "id": ["chord", ...] | "chord" }`.
pub fn parse_keybindings_json(raw: &str) -> Result<KeybindingsConfig, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "keybindings root must be an object".to_string())?;
    let mut out = KeybindingsConfig::new();
    for (id, v) in obj {
        let keys = match v {
            serde_json::Value::String(s) => vec![s.clone()],
            serde_json::Value::Array(arr) => {
                let mut keys = Vec::new();
                for item in arr {
                    let Some(s) = item.as_str() else {
                        return Err(format!("keybinding `{id}` array must be strings"));
                    };
                    keys.push(s.to_string());
                }
                keys
            }
            _ => {
                return Err(format!(
                    "keybinding `{id}` must be a string or array of strings"
                ));
            }
        };
        out.insert(id.clone(), keys);
    }
    Ok(out)
}

fn load_from_path(path: &Path) -> Result<KeybindingsConfig, String> {
    if !path.exists() {
        return Ok(KeybindingsConfig::new());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read failed: {e}"))?;
    parse_keybindings_json(&raw)
}

/// Build package `tui.*` + product `app.*` manager (no process-global write).
pub fn build_product_keybindings(user: KeybindingsConfig) -> KeybindingsManager {
    let mut defs = create_default_definitions();
    defs.extend(app_definitions());
    KeybindingsManager::new(defs, user)
}

/// Install package `tui.*` + product `app.*` with no disk overrides into the
/// process-global manager (demo / tests that intentionally touch globals).
pub fn install_product_keybindings_defaults_only() {
    set_keybindings(build_product_keybindings(KeybindingsConfig::new()));
}

/// Convenience: match a product/package id against the scoped or global manager.
pub fn matches_binding(event: &crossterm::event::KeyEvent, id: &'static str) -> bool {
    ensure_product_catalog();
    with_keybindings(|kb| kb.matches_event(event, id))
}

fn ensure_product_catalog() {
    let missing = with_keybindings(|kb| kb.get_definition("app.interrupt").is_none());
    if missing {
        install_product_keybindings_defaults_only();
    }
}

/// Load product keybindings from `agent_dir` without touching process globals.
pub fn load_product_keybindings(agent_dir: &Path) -> (KeybindingsManager, ReloadOutcome) {
    let path = keybindings_path(agent_dir);
    match load_from_path(&path) {
        Ok(user) => {
            let kb = build_product_keybindings(user);
            let outcome = if path.exists() {
                ReloadOutcome::Applied { path }
            } else {
                ReloadOutcome::NoFile { path }
            };
            (kb, outcome)
        }
        Err(error) => (
            build_product_keybindings(KeybindingsConfig::new()),
            ReloadOutcome::Failed { path, error },
        ),
    }
}

/// Install package `tui.*` + product `app.*` and apply disk overrides (if any)
/// into the process-global manager.
///
/// Prefer [`load_product_keybindings`] + [`xylitol_tui::KeybindingsScope`] for
/// HostSession so tests do not share one process-global writer.
#[cfg_attr(not(test), allow(dead_code))]
pub fn install_product_keybindings(agent_dir: &Path) -> ReloadOutcome {
    let (kb, outcome) = load_product_keybindings(agent_dir);
    set_keybindings(kb);
    outcome
}

/// Re-read `keybindings.json` into `kb`. On parse/IO failure, keep current bindings.
pub fn reload_keybindings_into(kb: &mut KeybindingsManager, agent_dir: &Path) -> ReloadOutcome {
    let path = keybindings_path(agent_dir);
    if !path.exists() {
        kb.set_user_bindings(KeybindingsConfig::new());
        return ReloadOutcome::NoFile { path };
    }
    match load_from_path(&path) {
        Ok(user) => {
            kb.set_user_bindings(user);
            ReloadOutcome::Applied { path }
        }
        Err(error) => ReloadOutcome::Failed { path, error },
    }
}

/// Re-read `keybindings.json` into the scoped or process-global manager.
/// On parse/IO failure, keep current bindings.
pub fn reload_keybindings(agent_dir: &Path) -> ReloadOutcome {
    with_keybindings_mut(|kb| reload_keybindings_into(kb, agent_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn parse_string_or_array() {
        let cfg = parse_keybindings_json(
            r#"{ "app.interrupt": "ctrl+x", "app.clear": ["ctrl+c", "ctrl+d"] }"#,
        )
        .unwrap();
        assert_eq!(cfg["app.interrupt"], vec!["ctrl+x".to_string()]);
        assert_eq!(
            cfg["app.clear"],
            vec!["ctrl+c".to_string(), "ctrl+d".to_string()]
        );
    }

    #[test]
    #[serial_test::serial(kb_global)]
    fn install_and_reload_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = install_product_keybindings(dir.path());
        assert!(matches!(outcome, ReloadOutcome::NoFile { .. }));
        assert!(matches_binding(
            &KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            "app.interrupt"
        ));

        let path = dir.path().join("keybindings.json");
        std::fs::write(&path, r#"{ "app.interrupt": ["ctrl+x"] }"#).unwrap();
        let outcome = reload_keybindings(dir.path());
        assert!(matches!(outcome, ReloadOutcome::Applied { .. }));
        assert!(!matches_binding(
            &KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            "app.interrupt"
        ));
        assert!(matches_binding(
            &KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            "app.interrupt"
        ));

        std::fs::write(&path, "{ not json").unwrap();
        let failed = reload_keybindings(dir.path());
        assert!(matches!(failed, ReloadOutcome::Failed { .. }));
        // Bad reload keeps previous successful override.
        assert!(matches_binding(
            &KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            "app.interrupt"
        ));
    }
}
