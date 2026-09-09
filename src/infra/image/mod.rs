//! Image processing for multimodal LLM input.
//!
//! - Scale to fit configurable max dimensions (default 2000×2000)
//! - Convert to JPEG with quality control when encoded size exceeds limit
//! - Produce base64 payload under ~4.5MB

mod error;
mod from_path;
mod resize;

pub use from_path::agent_part_from_image_path;
