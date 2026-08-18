//! Image decode / resize / path load failures (crate-private; not `Xy*`).

/// Multimodal image processing failures. Display keeps the original body.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ImageError(pub String);

impl From<&str> for ImageError {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for ImageError {
    fn from(value: String) -> Self {
        Self(value)
    }
}
