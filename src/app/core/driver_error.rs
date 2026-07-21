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

    /// Nested hot-path kind when [`Self::Agent`]; otherwise same as [`Self::kind`].
    pub fn detail_kind(&self) -> &'static str {
        match self {
            Self::Agent(inner) => inner.kind(),
            other => other.kind(),
        }
    }

    /// Log a driver/dispatch failure with stable `error.kind` (and `agent.kind` when nested).
    pub fn log_failure(&self, where_: &str) {
        match self {
            Self::Agent(inner) => {
                log::warn!(
                    target: "xylitol::driver",
                    "{where_} failed error.kind={} agent.kind={} error={self}",
                    self.kind(),
                    inner.kind()
                );
            }
            _ => {
                log::warn!(
                    target: "xylitol::driver",
                    "{where_} failed error.kind={} error={self}",
                    self.kind()
                );
            }
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

    /// Best-effort classify an opaque upstream string into a typed variant.
    ///
    /// Unknown strings stay [`Self::Message`] so Display text is unchanged.
    /// Matched strings keep the original body and add a kind prefix via Display.
    pub fn from_opaque(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        let lower = msg.to_ascii_lowercase();
        if lower.contains("not found")
            || lower.contains("no active session")
            || lower.contains("does not exist")
            || lower.contains("no such file")
            || lower.contains("no such session")
            || lower.contains("no models available")
        {
            Self::NotFound(msg)
        } else if lower.contains("not implemented")
            || lower.contains("unsupported")
            || lower.contains("not supported")
        {
            Self::Unsupported(msg)
        } else if lower.contains("invalid")
            || lower.contains("unknown ")
            || lower.starts_with("usage:")
            || lower.contains("unavailable")
        {
            Self::InvalidInput(msg)
        } else if lower.contains("permission denied")
            || lower.contains("i/o")
            || lower.contains("io error")
            || lower.contains("failed to read")
            || lower.contains("failed to write")
            || lower.contains("filesystem")
            || lower.contains("disk ")
            || lower.contains("clipboard")
            || lower.contains("trust store write")
        {
            Self::Io(msg)
        } else if lower.starts_with("remote:") || lower.contains("server error") {
            Self::Remote(msg)
        } else {
            Self::Message(msg)
        }
    }
}

impl From<String> for XyDriverError {
    fn from(value: String) -> Self {
        Self::from_opaque(value)
    }
}

impl From<&str> for XyDriverError {
    fn from(value: &str) -> Self {
        Self::from_opaque(value)
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
    fn detail_kind_for_agent_and_leaf() {
        let agent: XyDriverError = XyError::Aborted.into();
        assert_eq!(agent.kind(), "Agent");
        assert_eq!(agent.detail_kind(), "Aborted");
        let not_found = XyDriverError::not_found("x");
        assert_eq!(not_found.kind(), "NotFound");
        assert_eq!(not_found.detail_kind(), "NotFound");
    }

    #[test]
    fn kind_and_display_message() {
        let err = XyDriverError::message("opaque note");
        assert_eq!(err.kind(), "Message");
        assert_eq!(err.to_string(), "opaque note");
    }

    #[test]
    fn from_opaque_classifies_common_shapes() {
        let nf: XyDriverError = "session not found: abc".into();
        assert_eq!(nf.kind(), "NotFound");
        assert!(nf.to_string().contains("session not found: abc"));

        let un: XyDriverError = "session tree kind 'file_browser' is not implemented".into();
        assert_eq!(un.kind(), "Unsupported");
        assert!(un.to_string().contains("file_browser"));

        let inv: XyDriverError = "unknown theme `x`".into();
        assert_eq!(inv.kind(), "InvalidInput");

        let io: XyDriverError = "clipboard image task failed: boom".into();
        assert_eq!(io.kind(), "Io");

        let msg: XyDriverError =
            "This session has not been saved yet. Wait for the first assistant response.".into();
        assert_eq!(msg.kind(), "Message");
        assert!(msg.to_string().contains("not been saved yet"));
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
