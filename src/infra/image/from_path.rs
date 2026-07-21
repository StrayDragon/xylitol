//! Path → multimodal [`ImageContent`] (c1155 / i5).

use std::path::Path;

use crate::protocol::message::{AgentPart, ImageContent};

use super::resize::{ImageResizeOptions, resize_image};

/// Read an image file, resize for multimodal limits, return [`ImageContent`].
pub fn image_content_from_path(path: &Path) -> Result<ImageContent, String> {
    image_content_from_path_with_options(path, &ImageResizeOptions::default())
}

/// Same as [`image_content_from_path`] with custom resize options.
pub fn image_content_from_path_with_options(
    path: &Path,
    options: &ImageResizeOptions,
) -> Result<ImageContent, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read image {}: {e}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!("image file is empty: {}", path.display()));
    }
    let resized = resize_image(&bytes, options)?;
    Ok(ImageContent {
        url: None,
        data: Some(resized.data),
        media_type: resized.mime_type,
    })
}

/// Convenience: path → [`AgentPart::Image`].
pub fn agent_part_from_image_path(path: &Path) -> Result<AgentPart, String> {
    Ok(AgentPart::Image(image_content_from_path(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn small_png_path_yields_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tiny.png");
        let img: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_fn(8, 8, |_, _| Rgb([10, 20, 30]));
        img.save(&path).unwrap();
        let content = image_content_from_path(&path).unwrap();
        assert!(content.data.as_ref().is_some_and(|d| !d.is_empty()));
        assert!(content.media_type.starts_with("image/"));
    }
}
