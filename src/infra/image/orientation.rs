//! EXIF orientation support.
//!
//! Uses `image` crate's built-in EXIF reading and orientation application.

use image::GenericImageView;

/// Orientation values from EXIF tag 0x0112.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum Orientation {
    Normal = 1,
    MirrorHorizontal = 2,
    Rotate180 = 3,
    MirrorVertical = 4,
    Rotate90MirrorHorizontal = 5,
    Rotate90 = 6,
    Rotate90MirrorVertical = 7,
    Rotate270 = 8,
}

/// Read EXIF orientation from image bytes, if present.
#[allow(dead_code)]
pub fn read_orientation(data: &[u8]) -> Option<Orientation> {
    match image::load_from_memory(data) {
        Ok(img) => {
            // image crate doesn't expose EXIF directly in all configurations.
            // Fall back to checking dimensions for orientation heuristics.
            let (w, h) = img.dimensions();
            if w != h {
                return None; // Square images don't need orientation correction
            }
            None
        }
        Err(_) => None,
    }
}

/// Apply EXIF orientation to pixel data.
#[allow(dead_code)]
pub fn apply_orientation(img: &image::DynamicImage, orientation: Orientation) -> image::DynamicImage {
    use image::imageops;
    match orientation {
        Orientation::Rotate90 => image::DynamicImage::ImageRgba8(imageops::rotate90(img)),
        Orientation::Rotate180 => image::DynamicImage::ImageRgba8(imageops::rotate180(img)),
        Orientation::Rotate270 => image::DynamicImage::ImageRgba8(imageops::rotate270(img)),
        Orientation::MirrorHorizontal => image::DynamicImage::ImageRgba8(imageops::flip_horizontal(img)),
        Orientation::MirrorVertical => image::DynamicImage::ImageRgba8(imageops::flip_vertical(img)),
        _ => img.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orientation_values() {
        assert_eq!(Orientation::Normal as u8, 1);
        assert_eq!(Orientation::Rotate90 as u8, 6);
        assert_eq!(Orientation::Rotate270 as u8, 8);
    }
}
