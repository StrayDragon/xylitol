//! Image decode / resize / path load failures (crate-private; not `Xy*`).

use strum::IntoStaticStr;

/// Multimodal image processing failures. Display keeps the original body.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, IntoStaticStr)]
pub enum ImageError {
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Decode(String),
    #[error("{0}")]
    Empty(String),
    #[error("{0}")]
    Limit(String),
}

impl ImageError {
    pub fn kind(&self) -> &'static str {
        self.into()
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }

    pub fn decode(message: impl Into<String>) -> Self {
        Self::Decode(message.into())
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
        assert_eq!(ImageError::io("read").kind(), "Io");
        assert_eq!(ImageError::decode("png").kind(), "Decode");
        assert_eq!(ImageError::empty("0 bytes").kind(), "Empty");
        assert_eq!(ImageError::limit("too big").kind(), "Limit");
    }
}
