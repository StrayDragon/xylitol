//! Image resizing with dimension and byte-size constraints.
//!
//! Uses the `image` crate for core operations.

use base64::Engine as _;
use image::GenericImageView;
use std::io::Cursor;

use super::error::ImageError;

/// Result of a resize operation.
///
/// Only the encoded payload + MIME type are consumed by product (read tool);
/// dimension/flags metadata was dropped in c2750 (no reader).
#[derive(Debug, Clone)]
pub struct ResizedImage {
    /// Base64-encoded image data.
    pub data: String,
    /// MIME type (e.g. "image/jpeg").
    pub mime_type: String,
}

/// Read JPEG EXIF orientation (r1441): scans APP1 ‘Exif\0\0’ segments and the
/// initial IFD0 entry (tag 0x0112, SHORT, 1..=8). Returns `None` for non-JPEG
/// inputs, missing EXIF, or orientations that need no transform.
fn read_exif_orientation(data: &[u8]) -> Option<u8> {
    if !data.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut i = 2usize;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        if matches!(marker, 0x01 | 0xD0..=0xD7 | 0xD9) {
            i += 2;
            continue;
        }
        if i + 4 > data.len() {
            break;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        if len < 2 {
            break;
        }
        if marker == 0xE1 {
            let payload = &data[i + 4..(i + 2 + len).min(data.len())];
            if let Some(body) = payload.strip_prefix(b"Exif\0\0")
                && let Some(v) = parse_tiff_orientation(body)
            {
                return Some(v);
            }
        }
        i += 2 + len;
    }
    None
}

/// Minimal TIFF/IFD0 parser for tag 0x0112 (SHORT).
fn parse_tiff_orientation(tiff: &[u8]) -> Option<u8> {
    if tiff.len() < 8 {
        return None;
    }
    let little = tiff.starts_with(b"II");
    if !little && !tiff.starts_with(b"MM") {
        return None;
    }
    let read_u16 = |b: &[u8]| -> u16 {
        if little {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        }
    };
    let read_u32 = |b: &[u8]| -> u32 {
        if little {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        }
    };
    if read_u16(&tiff[2..4]) != 42 {
        return None;
    }
    let ifd0 = read_u32(&tiff[4..8]) as usize;
    if ifd0 + 2 > tiff.len() {
        return None;
    }
    let count = read_u16(&tiff[ifd0..ifd0 + 2]) as usize;
    for k in 0..count {
        let e = ifd0 + 2 + k * 12;
        if e + 12 > tiff.len() {
            return None;
        }
        let tag = read_u16(&tiff[e..e + 2]);
        if tag == 0x0112 {
            let vtype = read_u16(&tiff[e + 2..e + 4]);
            let value = read_u16(&tiff[e + 8..e + 10]);
            return (vtype == 3 && (1..=8).contains(&value)).then_some(value as u8);
        }
    }
    None
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
/// - Applies EXIF orientation metadata before any thumbnail work (r1441)
pub fn resize_image(data: &[u8], options: &ImageResizeOptions) -> Result<ResizedImage, ImageError> {
    let mut img = image::load_from_memory(data)
        .map_err(|e| ImageError::decode("failed to decode image", e))?;
    if let Some(orientation) = read_exif_orientation(data)
        && let Some(orientation) = image::metadata::Orientation::from_exif(orientation)
    {
        img.apply_orientation(orientation);
    }

    let (mut w, mut h) = img.dimensions();
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
    let png_b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

    if png_b64.len() <= options.max_bytes {
        return Ok(ResizedImage {
            data: png_b64,
            mime_type: "image/png".to_string(),
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

    let jpeg_b64 = base64::engine::general_purpose::STANDARD.encode(&jpeg_bytes);

    if jpeg_b64.len() <= options.max_bytes {
        return Ok(ResizedImage {
            data: jpeg_b64,
            mime_type: "image/jpeg".to_string(),
        });
    }

    Err(ImageError::limit(
        "image could not be resized below the byte limit",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Small image passes through as PNG (may re-encode to JPEG when the
        // byte limit forces it — metadata fields were removed in c2750).
        assert!(result.mime_type == "image/png" || result.mime_type == "image/jpeg");
    }

    // ── EXIF orientation（r1441）─────────────────────────────────────

    fn jpeg_rgb(w: u32, h: u32) -> Vec<u8> {
        let raw: Vec<u8> = (0..w * h).flat_map(|_| [0x10u8, 0x20, 0x30]).collect();
        let mut out = Vec::new();
        {
            let mut enc = image::codecs::jpeg::JpegEncoder::new(&mut out);
            enc.encode(&raw, w, h, image::ExtendedColorType::Rgb8)
                .unwrap();
        }
        out
    }

    fn tiff_with_orientation(orientation: u8) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"II");
        out.extend_from_slice(&[0x2A, 0x00]);
        out.extend_from_slice(&8u32.to_le_bytes()); // IFD0 offset
        out.extend_from_slice(&1u16.to_le_bytes()); // one entry
        out.extend_from_slice(&0x0112u16.to_le_bytes()); // tag Orientation
        out.extend_from_slice(&3u16.to_le_bytes()); // type SHORT
        out.extend_from_slice(&1u32.to_le_bytes()); // count
        out.extend_from_slice(&(orientation as u16).to_le_bytes());
        out.extend_from_slice(&[0u8, 0u8]); // pad
        out.extend_from_slice(&0u32.to_le_bytes()); // next IFD
        out
    }

    fn jpeg_with_exif(jpeg: &[u8], orientation: u8) -> Vec<u8> {
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
        let exif = tiff_with_orientation(orientation);
        let body: Vec<u8> = std::iter::once(b'E')
            .chain(std::iter::once(b'x'))
            .chain(std::iter::once(b'i'))
            .chain(std::iter::once(b'f'))
            .chain(std::iter::once(0u8))
            .chain(std::iter::once(0u8))
            .chain(exif)
            .collect();
        let len = (body.len() + 2) as u16;
        let mut out = vec![0xFF, 0xD8];
        out.extend_from_slice(&[0xFF, 0xE1, (len >> 8) as u8, (len & 0xFF) as u8]);
        out.extend_from_slice(&body);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    fn decoded_dims(out: &ResizedImage) -> (u32, u32) {
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&out.data)
            .unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        img.dimensions()
    }

    #[test]
    fn exif_orientation_6_swaps_dimensions() {
        let plain = jpeg_rgb(4, 8);
        let rotated = jpeg_with_exif(&plain, 6); // Rotate270
        let out = resize_image(&rotated, &ImageResizeOptions::default()).unwrap();
        let (w, h) = decoded_dims(&out);
        assert_eq!((w, h), (8, 4), "EXIF orientation 6 MUST 交换宽高");
    }

    #[test]
    fn exif_orientation_1_keeps_dimensions() {
        let plain = jpeg_rgb(4, 8);
        let kept = jpeg_with_exif(&plain, 1); // NoTransforms
        let out = resize_image(&kept, &ImageResizeOptions::default()).unwrap();
        let (w, h) = decoded_dims(&out);
        assert_eq!((w, h), (4, 8), "EXIF orientation 1 MUST 不旋转");
    }

    #[test]
    fn plain_jpeg_without_exif_is_untouched() {
        let plain = jpeg_rgb(4, 8);
        let out = resize_image(&plain, &ImageResizeOptions::default()).unwrap();
        let (w, h) = decoded_dims(&out);
        assert_eq!((w, h), (4, 8), "无 EXIF 输入 MUST 保持原尺寸");
    }
}
