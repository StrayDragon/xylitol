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

    // Note: the Input widget's cursor is reverse video (theme-independent),
    // owned entirely by `widgets::input::Input::render` — it does NOT live in
    // the palette. reverse swaps the terminal's default fg/bg, which reads
    // correctly on both dark and light themes without per-theme tuning, and a
    // trailing `\x1b[27m` in the glyph text scopes the reverse to one char so
    // it never leaks. This is the pi/kimi-code convention.
}

#[cfg(test)]
mod tests {
    use super::*;

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
