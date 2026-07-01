//! Semantic color tokens — the single source of truth for TUI color.
//!
//! Components MUST NOT hardcode color literals; they go through [`palette`].
//! Mirrors kimi-code `theme/colors.ts`: a small set of semantic tokens mapped
//! to ratatui `Style`, so the whole surface restyles from one place.

use ratatui_core::style::Color;
use ratatui_core::style::Modifier;
use ratatui_core::style::Style;

/// Semantic palette. Add tokens here, never inline `Color::...` in components.
pub fn palette() -> Palette {
    Palette
}

pub struct Palette;

impl Palette {
    /// Primary accent (titles, selected pointer, focus).
    #[allow(dead_code)]
    pub fn primary(&self) -> Style {
        Style::default().fg(Color::Cyan)
    }

    /// Dimmed/secondary text (hints, metadata).
    pub fn text_dim(&self) -> Style {
        Style::default().fg(Color::DarkGray)
    }

    /// Assistant message body.
    pub fn assistant(&self) -> Style {
        Style::default().fg(Color::Reset)
    }

    /// Tool execution labels.
    pub fn tool(&self) -> Style {
        Style::default().fg(Color::Yellow)
    }

    /// Error messages.
    pub fn error(&self) -> Style {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    }

    /// Spinner / in-progress indicator.
    #[allow(dead_code)]
    pub fn spinner(&self) -> Style {
        Style::default().fg(Color::Cyan)
    }

    /// Thinking / reasoning indicator (italic, dimmed) — pi-style "Thinking…".
    pub fn thinking(&self) -> Style {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC)
    }

    /// Subtle background for the input prompt line (pi-style Box bg block).
    pub fn input_bg(&self) -> Color {
        Color::Black
    }
}
