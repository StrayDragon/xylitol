//! Runtime keymap abstraction for the TUI.
//!
//! This is inspired by codex-rs' keymap system, but kept intentionally small
//! for xylitol's current TUI architecture.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AppKeyAction {
    ToggleShortcutOverlay,
    Quit,
    Interrupt,
    Clear,
    PreviewDiff,

    // New keys for codex parity (implemented in later tasks).
    OpenTranscript,
    CopyLastResponse,
    ToggleRawOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ComposerKeyAction {
    OpenExternalEditor,
    ShowHistorySearch,
}

/// A key binding with compatibility semantics.
///
/// Terminals disagree on whether SHIFT is preserved for printable characters.
/// We therefore match either the raw `KeyEvent` parts, or allow an alternate
/// representation that includes SHIFT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct KeyBinding {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyBinding {
    pub(crate) const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub(crate) const fn plain(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::NONE)
    }

    pub(crate) const fn ctrl(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::CONTROL)
    }

    pub(crate) const fn alt(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::ALT)
    }

    fn parts(&self) -> (KeyCode, KeyModifiers) {
        (self.code, self.modifiers)
    }

    pub(crate) fn matches(&self, key: &KeyEvent) -> bool {
        // Only handle key presses; ignore release/repeat.
        if key.kind != KeyEventKind::Press {
            return false;
        }

        let actual = (key.code, key.modifiers);
        if actual == self.parts() {
            return true;
        }

        // Compatibility: if the key is a printable char and the binding doesn't
        // require SHIFT, accept an equivalent SHIFTed chord.
        //
        // Example: terminals may report `?` as either:
        // - KeyCode::Char('?'), modifiers: NONE
        // - KeyCode::Char('?'), modifiers: SHIFT
        if let (KeyCode::Char(ch), expected_mods) = (self.code, self.modifiers) {
            if expected_mods.contains(KeyModifiers::SHIFT) {
                return false;
            }
            if matches!(key.code, KeyCode::Char(c) if c == ch)
                && key.modifiers == (expected_mods | KeyModifiers::SHIFT)
            {
                return true;
            }
        }

        false
    }
}

pub(crate) struct RuntimeKeymap {
    app: Vec<(KeyBinding, AppKeyAction)>,
    composer: Vec<(KeyBinding, ComposerKeyAction)>,
}

impl RuntimeKeymap {
    pub(crate) fn built_in_defaults() -> Self {
        use AppKeyAction as A;
        use ComposerKeyAction as C;

        let app = vec![
            (
                KeyBinding::plain(KeyCode::Char('?')),
                A::ToggleShortcutOverlay,
            ),
            (KeyBinding::ctrl(KeyCode::Char('d')), A::Quit),
            (KeyBinding::ctrl(KeyCode::Char('c')), A::Interrupt),
            (KeyBinding::ctrl(KeyCode::Char('l')), A::Clear),
            (KeyBinding::ctrl(KeyCode::Char('r')), A::PreviewDiff),
            // Codex parity defaults.
            (KeyBinding::ctrl(KeyCode::Char('t')), A::OpenTranscript),
            (KeyBinding::ctrl(KeyCode::Char('o')), A::CopyLastResponse),
            (KeyBinding::alt(KeyCode::Char('r')), A::ToggleRawOutput),
        ];

        let composer = vec![
            (KeyBinding::ctrl(KeyCode::Char('g')), C::OpenExternalEditor),
            (KeyBinding::ctrl(KeyCode::Char('r')), C::ShowHistorySearch),
        ];

        Self { app, composer }
    }

    pub(crate) fn resolve_app(&self, key: &KeyEvent) -> Option<AppKeyAction> {
        self.app
            .iter()
            .find_map(|(binding, action)| binding.matches(key).then_some(*action))
    }

    pub(crate) fn resolve_composer(&self, key: &KeyEvent) -> Option<ComposerKeyAction> {
        self.composer
            .iter()
            .find_map(|(binding, action)| binding.matches(key).then_some(*action))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_plain_char_with_optional_shift_modifier() {
        let binding = KeyBinding::plain(KeyCode::Char('?'));
        let key_plain = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        let key_shift = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);

        assert!(binding.matches(&key_plain));
        assert!(binding.matches(&key_shift));
    }

    #[test]
    fn does_not_match_wrong_modifier() {
        let binding = KeyBinding::ctrl(KeyCode::Char('d'));
        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        assert!(!binding.matches(&key));
    }
}
