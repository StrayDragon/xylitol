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

pub use image::{ClipboardImage, read_clipboard_image};
pub use native::copy_to_clipboard;
