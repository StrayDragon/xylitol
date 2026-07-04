//! Vendored ratatui-markdown core (c370).
//!
//! Derived from https://github.com/celestia-island/ratatui-markdown (Copyright
//! (c) 2026 langyo, SySL-1.0). Adapted to xylitol's ratatui-core split-crate
//! types. Only the markdown-rendering core is vendored (markdown + highlight +
//! theme + constants); mermaid/scroll/tree/preview/viewer/image are excluded.
//! See LICENSE + NOTICE in this directory for attribution.

// Vendored upstream code: silence lints that assume xylitol's conventions
// (unused re-exports are part of the upstream API surface; cfg gates for
// features we excluded produce unexpected_cfgs; style lints reflect upstream
// formatting, not xylitol's). The adapter layer in `components/markdown.rs`
// is where xylitol-specific lint rules apply.
#![allow(
    unused_imports,
    unexpected_cfgs,
    dead_code,
    clippy::collapsible_if,
    clippy::manual_range_contains,
    clippy::manual_checked_ops,
    clippy::too_many_arguments,
    clippy::manual_strip,
    clippy::needless_range_loop,
    clippy::useless_format
)]

pub mod constants;
pub mod highlight;
pub mod markdown;
pub mod theme;

#[allow(deprecated)]
pub use theme::DefaultTheme;
pub use theme::{CodeColors, RichTextTheme, ThemeBuilder, ThemeConfig};
