//! Image resizing with dimension and byte-size constraints.
//!
//! Uses the `image` crate for core operations.

use image::GenericImageView;
use std::io::Cursor;

use super::error::ImageError;

/// Result of a resize operation.
#[derive(Debug, Clone)]
pub struct ResizedImage {
    /// Base64-encoded image data.
    pub data: String,
    /// MIME type (e.g. "image/jpeg").
    pub mime_type: String,
    /// Original width in pixels.
    pub original_width: u32,
    /// Original height in pixels.
    pub original_height: u32,
    /// Final width in pixels.
    pub width: u32,
    /// Final height in pixels.
    pub height: u32,
    /// Whether the image was resized/converted.
    pub was_resized: bool,
}

/// Options for image resizing.
#[derive(Debug, Clone)]
pub struct ImageResizeOptions {
    pub max_width: u32,
    pub max_height: u32,
    /// Maximum encoded (base64) size in bytes. Default: 4.5MB (below Anthropic 5MB limit).
    pub max_bytes: usize,
    /// JPEG quality (1-100). Default: 80.
    pub jpeg_quality: u8,
}

impl Default for ImageResizeOptions {
    fn default() -> Self {
        Self {
            max_width: 2000,
            max_height: 2000,
            max_bytes: 4_500_000, // 4.5MB of base64 payload
            jpeg_quality: 80,
        }
    }
}

/// Resize an image to fit within constraints.
///
/// - Scales down if exceeding max dimensions (preserving aspect ratio)
/// - Converts to JPEG if PNG exceeds byte limit
///
/// Does not apply EXIF orientation correction; decoded pixels are used as-is.
pub fn resize_image(data: &[u8], options: &ImageResizeOptions) -> Result<ResizedImage, ImageError> {
    let img = image::load_from_memory(data)
        .map_err(|e| ImageError::decode("failed to decode image", e))?;

    let (orig_w, orig_h) = img.dimensions();
    let (mut w, mut h) = (orig_w, orig_h);
    let mut was_resized = false;

    // Scale down if needed
    if w > options.max_width || h > options.max_height {
        let ratio = (options.max_width as f64 / w as f64)
            .min(options.max_height as f64 / h as f64)
            .min(1.0);
        w = (w as f64 * ratio) as u32;
        h = (h as f64 * ratio) as u32;
        was_resized = true;
    }

    let resized = if was_resized {
        img.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
    } else {
        image::DynamicImage::ImageRgba8(img.to_rgba8())
    };

    // Try PNG first, then JPEG if too large
    let mut png_bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png_bytes);
        resized
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| ImageError::decode("failed to encode PNG", e))?;
    }
    let png_b64 = base64_encode(&png_bytes);

    if png_b64.len() <= options.max_bytes {
        return Ok(ResizedImage {
            data: png_b64,
            mime_type: "image/png".to_string(),
            original_width: orig_w,
            original_height: orig_h,
            width: w,
            height: h,
            was_resized,
        });
    }

    // Fallback to JPEG with quality control
    let rgb = resized.to_rgba8();
    let raw = rgb.as_raw();
    let mut jpeg_bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut jpeg_bytes);
        let mut jpeg_enc =
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, options.jpeg_quality);
        jpeg_enc
            .encode(raw, w, h, image::ExtendedColorType::Rgba8)
            .map_err(|e| ImageError::decode("failed to encode JPEG", e))?;
    }

    let jpeg_b64 = base64_encode(&jpeg_bytes);

    if jpeg_b64.len() <= options.max_bytes {
        was_resized = true;
        return Ok(ResizedImage {
            data: jpeg_b64,
            mime_type: "image/jpeg".to_string(),
            original_width: orig_w,
            original_height: orig_h,
            width: w,
            height: h,
            was_resized,
        });
    }

    Err(ImageError::limit(
        "image could not be resized below the byte limit",
    ))
}

/// Minimal base64 encoder (RFC 4648) — mirrors clipboard version.
fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn test_default_options() {
        let opts = ImageResizeOptions::default();
        assert_eq!(opts.max_width, 2000);
        assert_eq!(opts.max_height, 2000);
        assert_eq!(opts.max_bytes, 4_500_000);
        assert_eq!(opts.jpeg_quality, 80);
    }

    #[test]
    fn test_resize_small_image_stays_same() {
        // Create a small 10x10 PNG
        let mut buf = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buf);
            let img = image::DynamicImage::new_rgba8(10, 10);
            img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
        }

        let result = resize_image(&buf, &ImageResizeOptions::default()).unwrap();
        assert_eq!(result.original_width, 10);
        assert_eq!(result.original_height, 10);
        // Small image should not be resized
        assert!(!result.was_resized || result.mime_type == "image/jpeg");
    }
}
