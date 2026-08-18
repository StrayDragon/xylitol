use std::path::PathBuf;
use std::time::Duration;

use strum::IntoStaticStr;

#[derive(Debug, thiserror::Error, IntoStaticStr)]
pub enum XyError {
    #[error("provider error: {0}")]
    Provider(#[source] anyhow::Error),
    #[error("tool error: {0}")]
    Tool(#[from] XyToolError),
    #[error("session error: {0}")]
    Session(#[from] XySessionError),
    #[error("agent config error: {0}")]
    Config(String),
    #[error("aborted")]
    Aborted,
}

impl XyError {
    /// Stable kind for logs / fastrace (`Provider`, `Tool`, …).
    pub fn kind(&self) -> &'static str {
        self.into()
    }
}

#[derive(Debug, thiserror::Error, IntoStaticStr)]
pub enum XyToolError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(#[source] anyhow::Error),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("aborted")]
    Aborted,
}

impl XyToolError {
    /// Stable kind for logs / fastrace (`InvalidArgs`, `Aborted`, …).
    pub fn kind(&self) -> &'static str {
        self.into()
    }
}

/// Persistence failures from [`crate::protocol::ports::XySessionStore`].
///
/// Control-plane misses (no bound session, busy mutation, in-memory tree
/// travel) live on [`XySessionError`], not here.
#[derive(Debug, thiserror::Error, IntoStaticStr)]
pub enum XySessionStoreError {
    #[error("session not found: {session_id}")]
    NotFound { session_id: String },
    #[error("{op}: {source}")]
    Io {
        op: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("serialize entry: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("{message}")]
    Validation { message: String },
    #[error("{op} not supported")]
    Unsupported { op: &'static str },
}

/// Session-domain failures: store IO plus control-plane (no active session,
/// busy, in-memory tree travel).
#[derive(Debug, thiserror::Error, IntoStaticStr)]
pub enum XySessionError {
    #[error(transparent)]
    Store(#[from] XySessionStoreError),
    #[error("no active session")]
    NoActiveSession,
    #[error("{message}")]
    Busy { message: String },
    #[error("target entry not found: {entry_id}")]
    EntryNotFound { entry_id: String },
}

impl From<XySessionStoreError> for XyError {
    fn from(err: XySessionStoreError) -> Self {
        Self::Session(err.into())
    }
}

impl XySessionError {
    /// Persist variants keep the store kind (`NotFound`, `Io`, …); control-plane
    /// arms use their own (`NoActiveSession`, `Busy`, `EntryNotFound`).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Store(store) => store.kind(),
            other => other.into(),
        }
    }

    pub fn busy(message: impl Into<String>) -> Self {
        Self::Busy {
            message: message.into(),
        }
    }

    pub fn entry_not_found(entry_id: impl Into<String>) -> Self {
        Self::EntryNotFound {
            entry_id: entry_id.into(),
        }
    }
}

impl XySessionStoreError {
    pub fn kind(&self) -> &'static str {
        self.into()
    }

    pub fn not_found(session_id: impl Into<String>) -> Self {
        Self::NotFound {
            session_id: session_id.into(),
        }
    }

    pub fn io(op: &'static str, source: std::io::Error) -> Self {
        Self::Io { op, source }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn unsupported(op: &'static str) -> Self {
        Self::Unsupported { op }
    }
}

/// Filesystem (or equivalent) failures from [`crate::protocol::ports::XyExportIo`].
#[derive(Debug, thiserror::Error, IntoStaticStr)]
pub enum XyExportError {
    #[error("{op} {path}: {source}")]
    Io {
        op: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl XyExportError {
    pub fn kind(&self) -> &'static str {
        self.into()
    }

    pub fn io(op: &'static str, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            op,
            path: path.into(),
            source,
        }
    }
}

/// Persistence failures from [`crate::protocol::ports::XyTrustStore`].
#[derive(Debug, thiserror::Error, IntoStaticStr)]
pub enum XyTrustError {
    #[error("{op}: {source}")]
    Io {
        op: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("serialize trust store: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("failed to acquire trust store lock")]
    Lock,
}

impl XyTrustError {
    pub fn kind(&self) -> &'static str {
        self.into()
    }

    pub fn io(op: &'static str, source: std::io::Error) -> Self {
        Self::Io { op, source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── XyError Display ────────────────────────────────────────────

    #[test]
    fn xy_error_display_provider() {
        let err = XyError::Provider(anyhow::anyhow!("API returned 500"));
        assert_eq!(err.to_string(), "provider error: API returned 500");
    }

    #[test]
    fn xy_error_display_tool() {
        let err = XyError::Tool(XyToolError::ExecutionFailed(anyhow::anyhow!("disk full")));
        assert_eq!(err.to_string(), "tool error: execution failed: disk full");
    }

    #[test]
    fn xy_error_display_session() {
        let err = XyError::from(XySessionStoreError::not_found("abc"));
        assert_eq!(err.to_string(), "session error: session not found: abc");
    }

    #[test]
    fn xy_error_display_config() {
        let err = XyError::Config("missing api key".into());
        assert_eq!(err.to_string(), "agent config error: missing api key");
    }

    #[test]
    fn xy_error_display_aborted() {
        let err = XyError::Aborted;
        assert_eq!(err.to_string(), "aborted");
    }

    // ── XyToolError Display ─────────────────────────────────────────

    #[test]
    fn xy_tool_error_display_invalid_args() {
        let err = XyToolError::InvalidArgs("missing 'path'".into());
        assert_eq!(err.to_string(), "invalid arguments: missing 'path'");
    }

    #[test]
    fn xy_tool_error_display_execution_failed() {
        let err = XyToolError::ExecutionFailed(anyhow::anyhow!("permission denied"));
        assert_eq!(err.to_string(), "execution failed: permission denied");
    }

    #[test]
    fn xy_tool_error_display_permission_denied() {
        let err = XyToolError::PermissionDenied("/etc/shadow".into());
        assert_eq!(err.to_string(), "permission denied: /etc/shadow");
    }

    #[test]
    fn xy_tool_error_display_timeout() {
        let err = XyToolError::Timeout(Duration::from_secs(30));
        assert_eq!(err.to_string(), "timeout after 30s");
    }

    #[test]
    fn xy_tool_error_display_aborted() {
        let err = XyToolError::Aborted;
        assert_eq!(err.to_string(), "aborted");
    }

    // ── From conversion ─────────────────────────────────────────────

    #[test]
    fn xy_tool_error_into_xy_error() {
        let tool_err = XyToolError::Aborted;
        let err: XyError = tool_err.into();
        assert_eq!(err.to_string(), "tool error: aborted");
    }

    #[test]
    fn xy_tool_error_via_question_mark() {
        fn inner() -> Result<(), XyError> {
            Err(XyToolError::InvalidArgs("bad".into()))?
        }
        let result = inner();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "tool error: invalid arguments: bad"
        );
    }

    // ── kind() ──────────────────────────────────────────────────────

    #[test]
    fn xy_error_kind_variants() {
        assert_eq!(XyError::Provider(anyhow::anyhow!("x")).kind(), "Provider");
        assert_eq!(XyError::Tool(XyToolError::Aborted).kind(), "Tool");
        assert_eq!(
            XyError::from(XySessionStoreError::not_found("x")).kind(),
            "Session"
        );
        assert_eq!(XyError::Config("x".into()).kind(), "Config");
        assert_eq!(XyError::Aborted.kind(), "Aborted");
    }

    #[test]
    fn xy_tool_error_kind_variants() {
        assert_eq!(XyToolError::InvalidArgs("x".into()).kind(), "InvalidArgs");
        assert_eq!(
            XyToolError::ExecutionFailed(anyhow::anyhow!("x")).kind(),
            "ExecutionFailed"
        );
        assert_eq!(
            XyToolError::PermissionDenied("x".into()).kind(),
            "PermissionDenied"
        );
        assert_eq!(
            XyToolError::Timeout(Duration::from_secs(1)).kind(),
            "Timeout"
        );
        assert_eq!(XyToolError::Aborted.kind(), "Aborted");
    }

    #[test]
    fn xy_store_error_kind_and_display() {
        let nf = XySessionStoreError::not_found("abc");
        assert_eq!(nf.kind(), "NotFound");
        assert_eq!(nf.to_string(), "session not found: abc");
        assert_eq!(
            XySessionStoreError::unsupported("delete_session").to_string(),
            "delete_session not supported"
        );
        assert_eq!(
            XySessionError::NoActiveSession.to_string(),
            "no active session"
        );
        assert_eq!(XySessionError::NoActiveSession.kind(), "NoActiveSession");
        assert_eq!(
            XySessionError::busy("session mutation unavailable while busy").kind(),
            "Busy"
        );
    }

    #[test]
    fn xy_export_and_trust_error_kind() {
        let exp = XyExportError::io("write", "/tmp/out.html", std::io::Error::other("disk full"));
        assert_eq!(exp.kind(), "Io");
        assert!(exp.to_string().contains("write"));
        let trust = XyTrustError::Lock;
        assert_eq!(trust.kind(), "Lock");
        assert_eq!(trust.to_string(), "failed to acquire trust store lock");
    }

    // ── Debug ───────────────────────────────────────────────────────

    #[test]
    fn xy_error_debug_format() {
        let err = XyError::Config("oops".into());
        let debug = format!("{err:?}");
        assert!(debug.contains("Config"));
        assert!(debug.contains("oops"));
    }

    #[test]
    fn xy_tool_error_debug_format() {
        let err = XyToolError::Timeout(Duration::from_millis(500));
        let debug = format!("{err:?}");
        assert!(debug.contains("Timeout"));
    }
}
