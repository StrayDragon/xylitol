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
mod thinking_border;

pub use paint::{
    bg_rgb, bold, dim, fg_bg_rgb, fg_rgb, highlight_dollar_skill_refs, italic, mix_rgb,
    paint_left_rail_line, shade_toward_black, shade_toward_white, strikethrough, underline,
    word_wash_bg,
};
pub use palette::Palette;
pub use thinking_border::{ThinkingBorderLevel, apply_thinking_border};
