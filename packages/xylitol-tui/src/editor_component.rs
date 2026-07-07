use crate::tui::Component;

/// Interface for custom editor components.
///
/// Ported from pi's `editor-component.ts`. Extends [`Component`] with text
/// accessor/mutator methods plus optional history, autocomplete, and styling
/// hooks. Consumer can implement this trait to provide e.g. vim-mode,
/// emacs-mode, or custom-keybinding editors.
pub trait EditorComponent: Component {
    // ── core text access (required) ──────────────────────────────────────

    /// Current text content.
    fn get_text(&self) -> String;

    /// Set the text content.
    fn set_text(&mut self, text: String);

    // ── callbacks ────────────────────────────────────────────────────────

    /// Called when user submits (e.g. Enter).
    fn on_submit(&mut self, _text: &str) {}

    /// Called when text changes.
    fn on_change(&mut self, _text: &str) {}

    // ── history (optional) ───────────────────────────────────────────────

    /// Add text to history for up/down navigation.
    fn add_to_history(&mut self, _text: String) {}

    // ── advanced text manipulation (optional) ────────────────────────────

    /// Insert text at current cursor position.
    fn insert_text_at_cursor(&mut self, _text: &str) {}

    /// Get text with any markers expanded (e.g. paste markers).
    /// Falls back to [`get_text`] if not overridden.
    fn get_expanded_text(&self) -> String {
        self.get_text()
    }

    // ── appearance (optional) ────────────────────────────────────────────

    /// Border color function. Default identity.
    fn border_color(&self, s: &str) -> String {
        s.to_string()
    }

    /// Set horizontal padding.
    fn set_padding_x(&mut self, _padding: usize) {}
}
