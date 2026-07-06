//! Semantic color tokens — the single source of truth for TUI color.
//!
//! Components MUST NOT hardcode color literals; they go through [`palette`].
//! c399 stage 4: tokens returned the engine's own [`CellStyle`] / [`Color`]
//! (previously ratatui `Style`).
//!
//! Theme awareness: [`palette`] takes a [`TerminalTheme`] and every token
//! branches on it, so the UI adapts to a dark/light terminal the same way
//! the syntect code-highlighting did. The headline reason this exists is the
//! cursor: a raw `reverse` cursor on a transparent background swaps the
//! terminal's *default* fg/bg, producing a glaring block whose color the TUI
//! never chose. [`Palette::cursor`] returns an explicit fg+bg pair instead,
//! tuned per theme (light glyph on a dark block for dark terminals, the
//! inverse for light), so the cursor is readable on both without relying on
//! reverse semantics.

use crate::app::tui::engine::style::{CellStyle, Color};
use crate::app::tui::theme_detect::TerminalTheme;

/// Build the semantic palette for the given terminal theme. The [`Palette`]
/// carries the theme so each token can branch; pass the same instance around
/// the TUI rather than re-detecting per call.
pub fn palette(theme: TerminalTheme) -> Palette {
    Palette { theme }
}

/// Semantic palette. Add tokens here, never inline `Color::...` in components.
pub struct Palette {
    theme: TerminalTheme,
}

impl Palette {
    /// The resolved terminal theme this palette was built for.
    pub fn theme(&self) -> TerminalTheme {
        self.theme
    }

    /// Primary accent (titles, selected pointer, focus).
    #[allow(dead_code)]
    pub fn primary(&self) -> CellStyle {
        // Cyan reads on both themes; keep it stable.
        CellStyle::default().fg(Color::Cyan)
    }

    /// Dimmed/secondary text (hints, metadata, the greeting/status line).
    pub fn text_dim(&self) -> CellStyle {
        match self.theme {
            // DarkGray (90-range) vanishes on a light background.
            TerminalTheme::Dark => CellStyle::default().fg(Color::DarkGray),
            TerminalTheme::Light => CellStyle::default().fg(Color::Indexed(242)), // medium gray
        }
    }

    /// Assistant message body.
    pub fn assistant(&self) -> CellStyle {
        // Reset = terminal default foreground, correct on both themes.
        CellStyle::default().fg(Color::Reset)
    }

    /// User prompt echo (the `❯ <input>` line committed to scrollback on
    /// submit). Bold so the user can distinguish their input from the
    /// assistant reply in the scrollback history.
    pub fn user_prompt(&self) -> CellStyle {
        CellStyle::default().fg(Color::Cyan).bold()
    }

    /// Tool execution labels.
    pub fn tool(&self) -> CellStyle {
        // Yellow is readable on both; keep stable.
        CellStyle::default().fg(Color::Yellow)
    }

    /// Error messages.
    pub fn error(&self) -> CellStyle {
        CellStyle::default().fg(Color::Red).bold()
    }

    /// Spinner / in-progress indicator.
    #[allow(dead_code)]
    pub fn spinner(&self) -> CellStyle {
        CellStyle::default().fg(Color::Cyan)
    }

    /// Thinking / reasoning indicator (italic, dimmed) — pi-style "Thinking…".
    pub fn thinking(&self) -> CellStyle {
        match self.theme {
            TerminalTheme::Dark => CellStyle::default().fg(Color::DarkGray).italic(),
            TerminalTheme::Light => CellStyle::default().fg(Color::Indexed(242)).italic(),
        }
    }

    /// The fake cursor style used by the Input widget. Returns a fg-only style
    /// (NO bg) so the cursor highlights the glyph color without ever filling a
    /// background. This is the kimi-code philosophy: the palette has fg tokens
    /// only, the terminal background stays transparent throughout, and the
    /// cursor is just a high-contrast-colored character.
    ///
    /// Why no bg: SGR bg is sticky — once emitted it persists across subsequent
    /// spans until explicitly reset, so a bg on the cursor glyph would leak
    /// into the text and padding after it (visible as the background "following
    /// the cursor"). A fg-only cursor colors exactly one character and nothing
    /// else, regardless of cursor position.
    ///
    /// The cursor is bold too, so even where the fg color is close to the
    /// terminal default (rare), the weight distinguishes the cursor glyph.
    pub fn cursor(&self) -> CellStyle {
        match self.theme {
            // Bright white glyph on the (transparent) dark terminal.
            TerminalTheme::Dark => CellStyle::default().fg(Color::White).bold(),
            // Dark glyph on the (transparent) light terminal.
            TerminalTheme::Light => CellStyle::default().fg(Color::Black).bold(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_dark_is_fg_only_no_bg_no_reverse() {
        // The whole point: cursor colors the glyph with fg ONLY — no bg (which
        // would leak via sticky SGR into subsequent spans) and no reverse
        // (which swaps the terminal's uncontrolled default fg/bg).
        let p = palette(TerminalTheme::Dark);
        let c = p.cursor();
        assert_eq!(c.fg, Some(Color::White));
        assert_eq!(c.bg, None, "fg-only: no bg to leak");
        assert!(!c.reverse);
        assert!(c.bold, "bold distinguishes the cursor glyph");
    }

    #[test]
    fn cursor_light_is_fg_only_inverted() {
        let p = palette(TerminalTheme::Light);
        let c = p.cursor();
        assert_eq!(c.fg, Some(Color::Black));
        assert_eq!(c.bg, None, "fg-only: no bg to leak");
        assert!(!c.reverse);
        assert!(c.bold);
    }

    #[test]
    fn text_dim_branches_on_theme() {
        let dark = palette(TerminalTheme::Dark).text_dim().fg;
        let light = palette(TerminalTheme::Light).text_dim().fg;
        assert_eq!(dark, Some(Color::DarkGray));
        assert_eq!(light, Some(Color::Indexed(242)));
        assert_ne!(dark, light, "dim text must differ by theme");
    }

    #[test]
    fn thinking_branches_on_theme() {
        let dark = palette(TerminalTheme::Dark).thinking().fg;
        let light = palette(TerminalTheme::Light).thinking().fg;
        assert_ne!(dark, light);
    }

    #[test]
    fn stable_tokens_ignore_theme() {
        // These read fine on both; pin them so a future "per-theme accent"
        // refactor is a conscious change, not an accident.
        for t in [TerminalTheme::Dark, TerminalTheme::Light] {
            let p = palette(t);
            assert_eq!(p.primary().fg, Some(Color::Cyan));
            assert_eq!(p.assistant().fg, Some(Color::Reset));
            assert_eq!(p.tool().fg, Some(Color::Yellow));
            assert_eq!(p.error().fg, Some(Color::Red));
            assert_eq!(p.user_prompt().fg, Some(Color::Cyan));
            assert!(p.user_prompt().bold);
        }
    }

    #[test]
    fn theme_accessor_round_trips() {
        let p = palette(TerminalTheme::Light);
        assert_eq!(p.theme(), TerminalTheme::Light);
    }
}
