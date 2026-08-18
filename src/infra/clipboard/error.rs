//! Clipboard transport failures (crate-private; not `Xy*`).

/// Native tool / OSC 52 / join failures. Display keeps the original body.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ClipboardError(pub String);

impl From<&str> for ClipboardError {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for ClipboardError {
    fn from(value: String) -> Self {
        Self(value)
    }
}
