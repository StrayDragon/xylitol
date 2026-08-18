//! Clipboard transport failures (crate-private; not `Xy*`).

use strum::IntoStaticStr;

/// Native tool / OSC 52 / join failures. Display keeps the original body.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, IntoStaticStr)]
pub enum ClipboardError {
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    Decode(String),
}

impl ClipboardError {
    pub fn kind(&self) -> &'static str {
        self.into()
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported(message.into())
    }

    pub fn decode(message: impl Into<String>) -> Self {
        Self::Decode(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_error_kinds() {
        assert_eq!(ClipboardError::io("join").kind(), "Io");
        assert_eq!(
            ClipboardError::unsupported("no display").kind(),
            "Unsupported"
        );
        assert_eq!(ClipboardError::decode("b64").kind(), "Decode");
    }
}
