//! Clipboard operations — platform-native tools and OSC 52 fallback.
//!
//! Provides `copy_to_clipboard()` for text and `read_clipboard_image()`
//! for images, supporting macOS, Windows, Linux (Wayland + X11), and
//! remote sessions via OSC 52 escape sequences.
//!
//! This module is gated behind the `infra-clipboard` feature flag.

mod image;
mod native;
pub(crate) mod osc52;
mod text;

pub use image::{ClipboardImage, read_clipboard_image, write_clipboard_image_temp};
pub use native::{
    ClipboardPlan, apply_clipboard_plan_stdout, copy_to_clipboard, copy_to_clipboard_async,
    plan_clipboard_copy, plan_clipboard_copy_async,
};
pub use osc52::{format_osc52, is_remote_session};
pub use text::read_clipboard_text;
