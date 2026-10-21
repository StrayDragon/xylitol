mod error;
mod image;
mod native;
pub(crate) mod osc52;
mod text;

pub use error::ClipboardError;

pub use image::{read_clipboard_image, write_clipboard_image_temp};
pub use native::plan_clipboard_copy_async;
pub use text::read_clipboard_text;
