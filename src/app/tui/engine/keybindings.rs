//! Configurable keybinding registry (c399 stage 3).
//!
//! Port of pi's `keybindings.ts` (skill `ux.md` Step 3). Widgets bind to a
//! **keybinding id** (a `&'static str` like `"tui.input.submit"`) and query
//! [`KeybindingsManager::matches`]; they never hardcode key sequences. Users can
//! override defaults without forking widget code.
//!
//! ## What this is NOT (crossterm absorbs it)
//!
//! pi's `keys.ts` (~1400 lines) decodes three terminal protocols (legacy VT /
//! modifyOtherKeys / Kitty) into a `KeyId`. **crossterm already does this** —
//! `event::read()` yields a `KeyEvent { code, modifiers, kind }` normalized
//! across protocols (design.md D4). So this module works directly on
//! `KeyEvent`, no protocol decoding. We still need the registry because:
//! - widgets must not hardcode keys (project rule in AGENTS.md),
//! - users may override bindings (config),
//! - conflicts (one key claimed by two ids) must be surfaced, not silently shadowed.
//!
//! ## Key equality
//!
//! `KeyPattern` compares `code + modifiers` only (`kind` is ignored — release
//! events are filtered upstream). `Char('c')` with `CONTROL` ≠ `Char('c')` with
//! `NONE`, matching pi's `KeyId` semantics.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A keybinding identifier (dotted string, pi-style). `&'static str` so the
/// default registry can live in a `const` table without allocation.
pub type KeyId = &'static str;

/// A protocol-agnostic key pattern: `code + modifiers`, `kind`-agnostic.
/// Constructed from a crossterm `KeyEvent` or directly for defaults.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyPattern {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyPattern {
    /// A bare key with no modifiers (e.g. `KeyPattern::bare(KeyCode::Enter)`).
    pub const fn bare(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }
    /// A Ctrl+key combo.
    pub const fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }
    /// A Shift+key combo.
    pub const fn shift(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }
    /// Build from a crossterm `KeyEvent` (drops `kind`).
    pub fn from_event(ev: &KeyEvent) -> Self {
        Self {
            code: ev.code,
            modifiers: ev.modifiers,
        }
    }
}

/// A binding definition: id + one or more default patterns + human description.
#[derive(Clone, Debug)]
pub struct KeyBinding {
    pub id: KeyId,
    pub default: &'static [KeyPattern],
    pub description: &'static str,
}

/// The default keybinding table (stage-3 subset — covers what Input uses today
/// + app-level keys the host will bind via input listeners). Extend as widgets
/// grow; keep ids grouped by prefix (`tui.editor.*`, `tui.input.*`, `app.*`).
#[allow(clippy::doc_lazy_continuation)]
pub const DEFAULT_BINDINGS: &[KeyBinding] = &[
    // ── input / submit ───────────────────────────────────────────────────────
    KeyBinding {
        id: "tui.input.submit",
        default: &[KeyPattern::bare(KeyCode::Enter)],
        description: "submit the current input",
    },
    // ── app-level (host binds these as outcomes: abort / quit / redraw) ───────
    // pi: app.clear = ctrl+c, app.exit = ctrl+d. These stay on the widget's
    // handle_input path (pi model: the focused widget decides ctrl+c semantics,
    // not a global input listener — see ux.md Step 4 + tui.ts:825 comment).
    KeyBinding {
        id: "app.clear",
        default: &[KeyPattern::ctrl(KeyCode::Char('c'))],
        description: "abort the current turn (or clear input)",
    },
    KeyBinding {
        id: "app.exit",
        default: &[KeyPattern::ctrl(KeyCode::Char('d'))],
        description: "quit (when input is empty)",
    },
    KeyBinding {
        id: "app.force_redraw",
        default: &[KeyPattern::ctrl(KeyCode::Char('l'))],
        description: "force a full screen redraw",
    },
    // ── editor / input cursor movement + deletion ─────────────────────────────
    KeyBinding {
        id: "tui.editor.cursorLeft",
        default: &[KeyPattern::bare(KeyCode::Left)],
        description: "move cursor left one grapheme",
    },
    KeyBinding {
        id: "tui.editor.cursorRight",
        default: &[KeyPattern::bare(KeyCode::Right)],
        description: "move cursor right one grapheme",
    },
    KeyBinding {
        id: "tui.editor.cursorLineStart",
        default: &[KeyPattern::bare(KeyCode::Home)],
        description: "move cursor to line start",
    },
    KeyBinding {
        id: "tui.editor.cursorLineEnd",
        default: &[KeyPattern::bare(KeyCode::End)],
        description: "move cursor to line end",
    },
    KeyBinding {
        id: "tui.editor.deleteCharBackward",
        default: &[KeyPattern::bare(KeyCode::Backspace)],
        description: "delete the grapheme before the cursor",
    },
];

/// A conflict: one `KeyPattern` claimed by more than one id after merging user
/// overrides with defaults. The host surfaces these (config error), never
/// silently shadows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub pattern: KeyPattern,
    pub ids: Vec<KeyId>,
}

/// Resolved keybindings: defaults + user overrides, with conflict detection.
/// Widgets query via [`matches`]; the host queries [`get_conflicts`] after
/// loading user config to surface problems.
///
/// Not a global singleton (pi uses a process-global `getKeybindings()`) — each
/// [`Tui`](super::tui::Tui) owns one so tests can inject a manager with custom
/// overrides. The default (`Default::default()`) loads [`DEFAULT_BINDINGS`].
#[derive(Clone, Debug)]
pub struct KeybindingsManager {
    /// id → resolved patterns (user override replaces defaults if present).
    keys_by_id: HashMap<KeyId, Vec<KeyPattern>>,
    /// conflicts detected at `rebuild()` time.
    conflicts: Vec<Conflict>,
}

impl Default for KeybindingsManager {
    fn default() -> Self {
        let mut mgr = Self {
            keys_by_id: HashMap::new(),
            conflicts: Vec::new(),
        };
        mgr.rebuild(HashMap::new());
        mgr
    }
}

impl KeybindingsManager {
    /// True if `key` matches any resolved pattern for `id`. User overrides
    /// replace defaults (do not merge) — mirrors pi's `KeybindingsManager.matches`.
    pub fn matches(&self, key: &KeyEvent, id: KeyId) -> bool {
        let pat = KeyPattern::from_event(key);
        self.keys_by_id
            .get(id)
            .is_some_and(|pats| pats.contains(&pat))
    }

    /// Resolved patterns for an id (user override or defaults).
    pub fn get_keys(&self, id: KeyId) -> &[KeyPattern] {
        self.keys_by_id.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Apply user overrides + recompute conflicts. `overrides` maps id → the
    /// complete key list for that id (replaces the default, per pi semantics).
    pub fn rebuild(&mut self, overrides: HashMap<KeyId, Vec<KeyPattern>>) {
        self.keys_by_id.clear();
        // Seed defaults.
        for b in DEFAULT_BINDINGS {
            self.keys_by_id.insert(b.id, b.default.to_vec());
        }
        // Apply user overrides (replace).
        for (id, pats) in overrides {
            self.keys_by_id.insert(id, pats);
        }
        // Detect conflicts: pattern → set of ids claiming it.
        let mut owner: HashMap<KeyPattern, Vec<KeyId>> = HashMap::new();
        for (id, pats) in &self.keys_by_id {
            for p in pats {
                owner.entry(p.clone()).or_default().push(id);
            }
        }
        self.conflicts = owner
            .into_iter()
            .filter_map(|(pattern, mut ids)| {
                if ids.len() > 1 {
                    ids.sort();
                    Some(Conflict { pattern, ids })
                } else {
                    None
                }
            })
            .collect();
        self.conflicts
            .sort_by(|a, b| format!("{:?}", a.pattern).cmp(&format!("{:?}", b.pattern)));
    }

    /// Conflicts found in the current configuration (empty if none).
    pub fn get_conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    /// All resolved bindings (for diagnostics / a future `/bindings` command).
    pub fn get_resolved(&self) -> &HashMap<KeyId, Vec<KeyPattern>> {
        &self.keys_by_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }
    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }
    fn bare_char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    // ── matches ───────────────────────────────────────────────────────────────

    #[test]
    fn matches_default_submit_on_enter() {
        let kb = KeybindingsManager::default();
        assert!(kb.matches(&key(KeyCode::Enter, KeyModifiers::NONE), "tui.input.submit"));
    }

    #[test]
    fn matches_app_clear_on_ctrl_c() {
        let kb = KeybindingsManager::default();
        assert!(kb.matches(&ctrl_char('c'), "app.clear"));
        // bare 'c' (no ctrl) must NOT match.
        assert!(!kb.matches(&bare_char('c'), "app.clear"));
    }

    #[test]
    fn no_match_for_unbound_key() {
        let kb = KeybindingsManager::default();
        assert!(!kb.matches(&bare_char('q'), "tui.input.submit"));
    }

    #[test]
    fn unknown_id_never_matches() {
        let kb = KeybindingsManager::default();
        assert!(!kb.matches(&bare_char('a'), "does.not.exist"));
        assert!(kb.get_keys("does.not.exist").is_empty());
    }

    #[test]
    fn matches_cursor_movement_defaults() {
        let kb = KeybindingsManager::default();
        assert!(kb.matches(
            &key(KeyCode::Left, KeyModifiers::NONE),
            "tui.editor.cursorLeft"
        ));
        assert!(kb.matches(
            &key(KeyCode::Right, KeyModifiers::NONE),
            "tui.editor.cursorRight"
        ));
        assert!(kb.matches(
            &key(KeyCode::Home, KeyModifiers::NONE),
            "tui.editor.cursorLineStart"
        ));
        assert!(kb.matches(
            &key(KeyCode::End, KeyModifiers::NONE),
            "tui.editor.cursorLineEnd"
        ));
        assert!(kb.matches(
            &key(KeyCode::Backspace, KeyModifiers::NONE),
            "tui.editor.deleteCharBackward"
        ));
    }

    // ── user overrides ────────────────────────────────────────────────────────

    #[test]
    fn user_override_replaces_default() {
        let mut kb = KeybindingsManager::default();
        // Remap submit to Ctrl+Enter.
        let mut overrides = HashMap::new();
        overrides.insert("tui.input.submit", vec![KeyPattern::ctrl(KeyCode::Enter)]);
        kb.rebuild(overrides);
        // Enter alone no longer submits.
        assert!(!kb.matches(&key(KeyCode::Enter, KeyModifiers::NONE), "tui.input.submit"));
        // Ctrl+Enter does.
        assert!(kb.matches(&ctrl_key(KeyCode::Enter), "tui.input.submit"));
    }

    #[test]
    fn user_override_multiple_keys() {
        let mut kb = KeybindingsManager::default();
        // cancel via escape OR ctrl+c.
        let mut overrides = HashMap::new();
        overrides.insert(
            "app.clear",
            vec![
                KeyPattern::bare(KeyCode::Esc),
                KeyPattern::ctrl(KeyCode::Char('c')),
            ],
        );
        kb.rebuild(overrides);
        assert!(kb.matches(&key(KeyCode::Esc, KeyModifiers::NONE), "app.clear"));
        assert!(kb.matches(&ctrl_char('c'), "app.clear"));
    }

    // ── conflict detection ────────────────────────────────────────────────────

    #[test]
    fn no_conflicts_with_defaults() {
        let kb = KeybindingsManager::default();
        assert!(kb.get_conflicts().is_empty(), "defaults have no overlaps");
    }

    #[test]
    fn detects_user_introduced_conflict() {
        let mut kb = KeybindingsManager::default();
        // Bind both submit and force_redraw to Enter → conflict.
        let mut overrides = HashMap::new();
        overrides.insert("tui.input.submit", vec![KeyPattern::bare(KeyCode::Enter)]);
        overrides.insert("app.force_redraw", vec![KeyPattern::bare(KeyCode::Enter)]);
        kb.rebuild(overrides);
        let conflicts = kb.get_conflicts();
        assert_eq!(conflicts.len(), 1, "one conflicting pattern");
        assert_eq!(conflicts[0].pattern, KeyPattern::bare(KeyCode::Enter));
        assert!(conflicts[0].ids.contains(&"tui.input.submit"));
        assert!(conflicts[0].ids.contains(&"app.force_redraw"));
    }

    #[test]
    fn conflict_clears_after_rebuild_without_overlap() {
        let mut kb = KeybindingsManager::default();
        let mut bad = HashMap::new();
        bad.insert("app.force_redraw", vec![KeyPattern::bare(KeyCode::Enter)]);
        kb.rebuild(bad);
        assert!(!kb.get_conflicts().is_empty());
        // Rebuild with a non-conflicting override.
        kb.rebuild(HashMap::new());
        assert!(kb.get_conflicts().is_empty());
    }

    // ── KeyPattern semantics ──────────────────────────────────────────────────

    #[test]
    fn pattern_eq_ignores_kind_compares_code_modifiers() {
        // KeyEvent has a `kind` field (Press/Release/Repeat); KeyPattern drops it.
        let press = KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            crossterm::event::KeyEventKind::Press,
        );
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            crossterm::event::KeyEventKind::Release,
        );
        assert_eq!(
            KeyPattern::from_event(&press),
            KeyPattern::from_event(&release)
        );
    }

    #[test]
    fn pattern_distinguishes_modifiers() {
        let bare = KeyPattern::bare(KeyCode::Char('c'));
        let ctrl = KeyPattern::ctrl(KeyCode::Char('c'));
        assert_ne!(bare, ctrl);
    }

    fn ctrl_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }
}
