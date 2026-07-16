//! Image processing utilities — resize, EXIF orientation, format conversion.
//!
//! Optimizes images for multimodal LLM input:
//! - Scales to fit within configurable max dimensions (default 2000×2000)
//! - Applies EXIF orientation
//! - Converts to JPEG with quality control when size exceeds limit
//! - Produces base64-encoded payload under 4.5MB

mod format;
mod from_path;
mod orientation;
mod resize;

pub use from_path::{agent_part_from_image_path, image_content_from_path};
pub use resize::resize_image;
pub use resize::{ImageResizeOptions, ResizedImage};
