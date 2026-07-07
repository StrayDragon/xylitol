//! Semantic color tokens — the single source of truth for TUI color.
//!
//! Components MUST NOT hardcode color literals; they go through [`palette`].
//! c399 stage 4: tokens now return the engine's own [`CellStyle`] / [`Color`]
//! (previously ratatui `Style`). The whole TUI no longer depends on ratatui.

use crate::app::tui::engine::style::{CellStyle, Color};

/// Semantic palette. Add tokens here, never inline `Color::...` in components.
pub fn palette() -> Palette {
    Palette
}

pub struct Palette;

impl Palette {
    /// Primary accent (titles, selected pointer, focus).
    #[allow(dead_code)]
    pub fn primary(&self) -> CellStyle {
        CellStyle::default().fg(Color::Cyan)
    }

    /// Dimmed/secondary text (hints, metadata).
    pub fn text_dim(&self) -> CellStyle {
        CellStyle::default().fg(Color::DarkGray)
    }

    /// Assistant message body.
    pub fn assistant(&self) -> CellStyle {
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
        CellStyle::default().fg(Color::DarkGray).italic()
    }
}
