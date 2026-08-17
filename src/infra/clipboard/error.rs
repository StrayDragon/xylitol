//! Clipboard transport failures (crate-private; not `Xy*`).

use std::io;

use strum::IntoStaticStr;

/// Native tool / OSC 52 / join failures. Display keeps the original body.
#[derive(Debug, thiserror::Error, IntoStaticStr)]
pub enum ClipboardError {
    /// Native clipboard tool / filesystem failure (source preserved).
    #[error("{context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },
    /// Blocking-pool join failure — opaque payload only.
    #[error("{0}")]
    Join(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    Decode(String),
}

impl ClipboardError {
    pub fn kind(&self) -> &'static str {
        self.into()
    }

    pub fn io(context: &'static str, source: io::Error) -> Self {
        Self::Io { context, source }
    }

    pub fn join(message: impl Into<String>) -> Self {
        Self::Join(message.into())
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
        assert_eq!(
            ClipboardError::io("join", io::Error::other("boom")).kind(),
            "Io"
        );
        assert_eq!(
            ClipboardError::unsupported("no display").kind(),
            "Unsupported"
        );
        assert_eq!(ClipboardError::decode("b64").kind(), "Decode");
    }
}
