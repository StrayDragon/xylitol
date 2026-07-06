//! pi-tui widget catalog (c399, skill `ui-components.md` Step 5).
//!
//! Built-ins on the `Component` contract (`engine::component`). Each widget
//! honors the width contract (render(width) lines ≤ width), re-opens styles per
//! line, never writes to the terminal directly, and caches render output keyed
//! by (content signature, width) — cleared on `invalidate()`.
//!
//! Build order (see `tasks.md` stage 2):
//! 1. `text` — Text (wrap + cache), Spacer, TruncatedText
//! 2. `markdown` — passthrough + code-block highlight, with pluggable syntax
//!    handlers (the "knobs" for gradually enabling per-element styling)
//! 3. `input` — single-line Focusable input
//! 4. `loader` — spinner

pub mod input;
pub mod loader;
pub mod markdown;
pub mod text;
