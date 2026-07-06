//! Adapter: ratatui `Style` → engine `CellStyle` (c399 transitional).
//!
//! The c399 stage-2 Markdown widget reuses the c396 `syntect_highlight` layer,
//! whose `StyleSegment` carries a `ratatui_core::style::Style`. Until c399 stage
//! 4 fully removes ratatui (and the syntect layer is rewritten to emit our
//! `CellStyle` directly), this adapter bridges the two.
//!
//! This module is THIN and TEMPORARY: it exists so stage 2 can reuse c396's
//! battle-tested highlighter without rewriting it mid-stream. When stage 4
//! removes ratatui, `syntect_highlight` will be ported to emit `CellStyle` and
//! this adapter deleted.

use crate::app::tui::engine::style::{CellStyle, Color};

/// Wrapper around ratatui's Style so we can define a conversion without a
/// foreign crate's trait. (Ratatui Style is Copy, so this is cheap.)
pub struct RatatuiStyle(pub ratatui_core::style::Style);

impl RatatuiStyle {
    /// Convert to the engine's `CellStyle`.
    pub fn into_cell_style(self) -> CellStyle {
        let s = self.0;
        CellStyle {
            fg: s.fg.map(convert_color),
            bg: s.bg.map(convert_color),
            bold: s.add_modifier.contains(ratatui_core::style::Modifier::BOLD),
            italic: s
                .add_modifier
                .contains(ratatui_core::style::Modifier::ITALIC),
            underline: s
                .add_modifier
                .contains(ratatui_core::style::Modifier::UNDERLINED),
            dim: s.add_modifier.contains(ratatui_core::style::Modifier::DIM),
            crossed_out: s
                .add_modifier
                .contains(ratatui_core::style::Modifier::CROSSED_OUT),
            reverse: s
                .add_modifier
                .contains(ratatui_core::style::Modifier::REVERSED),
        }
    }
}

impl From<ratatui_core::style::Style> for RatatuiStyle {
    fn from(s: ratatui_core::style::Style) -> Self {
        Self(s)
    }
}

/// The engine's CellStyle accepts a ratatui Style directly via this conversion
/// (used by the markdown widget's style_at_byte). Defined here to keep the
/// ratatui dependency in this adapter module only.
impl From<ratatui_core::style::Style> for CellStyle {
    fn from(s: ratatui_core::style::Style) -> Self {
        RatatuiStyle(s).into_cell_style()
    }
}

fn convert_color(c: ratatui_core::style::Color) -> Color {
    use ratatui_core::style::Color as R;
    match c {
        R::Black => Color::Black,
        R::Red => Color::Red,
        R::Green => Color::Green,
        R::Yellow => Color::Yellow,
        R::Blue => Color::Blue,
        R::Magenta => Color::Magenta,
        R::Cyan => Color::Cyan,
        R::Gray => Color::Gray,
        R::DarkGray => Color::DarkGray,
        R::LightRed => Color::LightRed,
        R::LightGreen => Color::LightGreen,
        R::LightYellow => Color::LightYellow,
        R::LightBlue => Color::LightBlue,
        R::LightMagenta => Color::LightMagenta,
        R::LightCyan => Color::LightCyan,
        R::White => Color::White,
        R::Indexed(n) => Color::Indexed(n),
        R::Rgb(r, g, b) => Color::Rgb(r, g, b),
        R::Reset => Color::Reset,
    }
}
