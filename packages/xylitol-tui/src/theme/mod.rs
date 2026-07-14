//! Optional semantic theme helpers for hosts (demo / app).
//!
//! Components still take closure `*Theme` only. This module is a **typed palette**
//! plus paint helpers — not a TypeScript-style Theme service / JSON market.
//!
//! ```ignore
//! let p = Palette::from(TerminalColorScheme::Light);
//! let md = p.markdown_theme();
//! ```

mod paint;
mod palette;

pub use paint::{
    bg_rgb, bold, dim, fg_bg_rgb, fg_rgb, italic, mix_rgb, shade_toward_black, shade_toward_white,
    strikethrough, underline, word_wash_bg,
};
pub use palette::Palette;

/// Alias kept for call sites that already say “semantic”.
pub type SemanticPalette = Palette;
