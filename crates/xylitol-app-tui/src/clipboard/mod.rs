//! Clipboard operations — platform-native tools and OSC 52 fallback.
//!
//! Provides `copy_to_clipboard()` for text and `read_clipboard_image()`
//! for images, supporting macOS, Windows, Linux (Wayland + X11), and
//! remote sessions via OSC 52 escape sequences.
//!
//! This module is gated behind the `infra-clipboard` feature flag.

mod error;
mod image;
mod native;
pub(crate) mod osc52;
mod text;

pub use error::ClipboardError;

pub use image::{read_clipboard_image, write_clipboard_image_temp};
pub use native::plan_clipboard_copy_async;
pub use text::read_clipboard_text;
