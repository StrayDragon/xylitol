//! Input widget — single-line Focusable text input (c399 stage 2.3).
//!
//! Placeholder for stage 2.3. The current focus is the engine + Text + Markdown;
//! Input arrives before stage 4 (main-loop integration) since the chat UI needs
//! it. For now this module exists so `widgets/mod.rs` resolves.

// TODO(c399 stage 2.3): single-line Focusable input — horizontal scroll,
// grapheme cursor, CURSOR_MARKER emission, reuse existing cursor_x_at CJK logic.
