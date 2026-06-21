//! Image format detection and conversion.

/// Detect image MIME type from magic bytes.
#[allow(dead_code)]
pub fn detect_mime_type(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if data.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if data.starts_with(b"RIFF") && data.len() > 12 && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else if data.starts_with(b"GIF8") {
        Some("image/gif")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_png() {
        let png_header = b"\x89PNG\r\n\x1a\n";
        assert_eq!(detect_mime_type(png_header), Some("image/png"));
    }

    #[test]
    fn test_detect_jpeg() {
        let jpeg_header = b"\xff\xd8\xff\xe0";
        assert_eq!(detect_mime_type(jpeg_header), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(detect_mime_type(b"unknown"), None);
    }
}
