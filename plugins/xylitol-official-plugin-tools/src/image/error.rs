//! Image decode / resize / path load failures (crate-private; not `Xy*`).

use strum::IntoStaticStr;

/// Multimodal image processing failures. Display keeps the original body.
#[derive(Debug, thiserror::Error, IntoStaticStr)]
pub enum ImageError {
    /// Reading the image file failed (source preserved).
    #[error("read image {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// Decode / encode by the `image` crate failed (source preserved).
    #[error("{context}: {source}")]
    Decode {
        context: &'static str,
        #[source]
        source: image::ImageError,
    },
    #[error("{0}")]
    Empty(String),
    #[error("{0}")]
    Limit(String),
}

impl ImageError {
    pub fn kind(&self) -> &'static str {
        self.into()
    }

    pub fn io(path: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn decode(context: &'static str, source: image::ImageError) -> Self {
        Self::Decode { context, source }
    }

    pub fn empty(message: impl Into<String>) -> Self {
        Self::Empty(message.into())
    }

    pub fn limit(message: impl Into<String>) -> Self {
        Self::Limit(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_error_kinds() {
        assert_eq!(
            ImageError::io("/tmp/a", std::io::Error::other("read")).kind(),
            "Io"
        );
        assert_eq!(
            ImageError::decode(
                "decode",
                image::ImageError::IoError(std::io::Error::other("png"))
            )
            .kind(),
            "Decode"
        );
        assert_eq!(ImageError::empty("0 bytes").kind(), "Empty");
        assert_eq!(ImageError::limit("too big").kind(), "Limit");
    }
}
