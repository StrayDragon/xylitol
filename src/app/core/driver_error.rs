//! [`XyDriverError`] — typed failures for the shared application driver protocol.

use crate::protocol::error::{XyError, XyToolError};

/// Errors from [`super::driver::XyDriver`] (整机遥控器 / 多面共享应用协议).
///
/// Distinct from [`XyError`] (ReAct / provider / tool hot path). Agent-loop
/// failures that surface through the driver are wrapped as [`Self::Agent`].
#[derive(Debug, thiserror::Error)]
pub enum XyDriverError {
    /// Resource missing (session, entry, path, …).
    #[error("not found: {0}")]
    NotFound(String),

    /// This driver backend does not support the capability.
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// Caller input / argument invalid.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Local IO / filesystem / clipboard transport failure.
    #[error("io: {0}")]
    Io(String),

    /// Remote driver / HTTP envelope failure.
    #[error("remote: {0}")]
    Remote(String),

    /// Agent / provider / tool failure bubbled through the driver.
    #[error(transparent)]
    Agent(#[from] XyError),

    /// Display-preserving message (migration / opaque upstream strings).
    ///
    /// Prefer a more specific variant at new call sites. Kept so existing
    /// human-readable strings stay stable while signatures move off `String`.
    #[error("{0}")]
    Message(String),
}

impl XyDriverError {
    /// Stable kind for logs / fastrace (`NotFound`, `Agent`, …).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "NotFound",
            Self::Unsupported(_) => "Unsupported",
            Self::InvalidInput(_) => "InvalidInput",
            Self::Io(_) => "Io",
            Self::Remote(_) => "Remote",
            Self::Agent(_) => "Agent",
            Self::Message(_) => "Message",
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::Unsupported(msg.into())
    }

    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    pub fn io(msg: impl Into<String>) -> Self {
        Self::Io(msg.into())
    }

    pub fn remote(msg: impl Into<String>) -> Self {
        Self::Remote(msg.into())
    }

    pub fn message(msg: impl Into<String>) -> Self {
        Self::Message(msg.into())
    }
}

impl From<String> for XyDriverError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for XyDriverError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

impl From<XyToolError> for XyDriverError {
    fn from(value: XyToolError) -> Self {
        Self::Agent(XyError::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_and_display_message() {
        let err = XyDriverError::message("session not found: abc");
        assert_eq!(err.kind(), "Message");
        assert_eq!(err.to_string(), "session not found: abc");
    }

    #[test]
    fn from_xy_error() {
        let err: XyDriverError = XyError::Aborted.into();
        assert_eq!(err.kind(), "Agent");
        assert_eq!(err.to_string(), "aborted");
    }

    #[test]
    fn not_found_display() {
        let err = XyDriverError::not_found("session xyz");
        assert_eq!(err.to_string(), "not found: session xyz");
        assert_eq!(err.kind(), "NotFound");
    }
}
