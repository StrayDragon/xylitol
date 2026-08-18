//! [`XyDriverError`] — typed failures for the shared application driver protocol.

use strum::IntoStaticStr;

use crate::agent::compaction::CompactionError;
use crate::agent::runtime::RuntimeControlError;
use crate::infra::clipboard::ClipboardError;
use crate::infra::config::error::LoadError;
use crate::infra::mcp::McpError;
use crate::protocol::error::{XyError, XyExportError, XyStoreError, XyToolError, XyTrustError};

/// Errors from [`super::driver::XyDriver`] (整机遥控器 / 多面共享应用协议).
///
/// Distinct from [`XyError`] (ReAct / provider / tool hot path). Agent-loop
/// failures that surface through the driver are wrapped as [`Self::Agent`].
#[derive(Debug, thiserror::Error, IntoStaticStr)]
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
    Agent(XyError),

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
        self.into()
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
    ///
    /// Prefer specific phrases over bare tokens (`unavailable`, `failed to`) to
    /// avoid misclassifying status notes and business-logic messages.
    pub fn from_opaque(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        let lower = msg.to_ascii_lowercase();
        if lower.contains("not found")
            || lower.contains("no active session")
            || lower.contains("does not exist")
            || lower.contains("no such file")
            || lower.contains("no such session")
            || lower.contains("no models available")
            || lower.contains("no pending entries")
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
            || lower.contains("trust option unavailable")
            || lower.contains("no thinking levels")
            || lower.contains("set $visual")
            || lower.contains("set $editor")
        {
            Self::InvalidInput(msg)
        } else if lower.contains("permission denied")
            || lower.contains("i/o")
            || lower.contains("io error")
            || lower.contains("failed to read")
            || lower.contains("failed to write")
            || lower.contains("failed to list")
            || lower.contains("failed to acquire")
            || lower.contains("failed to stat")
            || lower.contains("read sessions dir")
            || lower.contains("filesystem")
            || lower.contains("disk ")
            || lower.contains("clipboard")
            || lower.contains("trust store write")
            || lower.contains("create trust lock")
            || lower.contains("write tempfile")
            || lower.contains("spawn ")
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

impl From<XyError> for XyDriverError {
    fn from(err: XyError) -> Self {
        match err {
            XyError::Session(store) => store.into(),
            other => Self::Agent(other),
        }
    }
}

impl From<XyToolError> for XyDriverError {
    fn from(value: XyToolError) -> Self {
        Self::Agent(XyError::from(value))
    }
}

impl From<XyStoreError> for XyDriverError {
    fn from(err: XyStoreError) -> Self {
        match err {
            XyStoreError::NotFound { session_id } => Self::not_found(session_id),
            XyStoreError::EntryNotFound { entry_id } => Self::not_found(entry_id),
            XyStoreError::NoActiveSession => Self::not_found("no active session"),
            XyStoreError::Io { op, source } => Self::io(format!("{op}: {source}")),
            XyStoreError::Serialize(source) => Self::io(format!("serialize entry: {source}")),
            XyStoreError::Unsupported { op } => Self::unsupported(format!("{op} not supported")),
            XyStoreError::Validation { message } => Self::message(message),
        }
    }
}

impl From<XyExportError> for XyDriverError {
    fn from(err: XyExportError) -> Self {
        match err {
            XyExportError::Io { op, path, source } => {
                Self::io(format!("{op} {}: {source}", path.display()))
            }
        }
    }
}

impl From<XyTrustError> for XyDriverError {
    fn from(err: XyTrustError) -> Self {
        match err {
            XyTrustError::Io { op, source } => Self::io(format!("{op}: {source}")),
            XyTrustError::Parse(source) => Self::io(format!("serialize trust store: {source}")),
            XyTrustError::Lock => Self::io("failed to acquire trust store lock"),
        }
    }
}

impl From<CompactionError> for XyDriverError {
    fn from(err: CompactionError) -> Self {
        match err {
            CompactionError::Store(store) => store.into(),
            CompactionError::Policy(message) => Self::message(message),
        }
    }
}

impl From<RuntimeControlError> for XyDriverError {
    fn from(err: RuntimeControlError) -> Self {
        match err {
            RuntimeControlError::NoSession => Self::not_found("no active session"),
            RuntimeControlError::Busy | RuntimeControlError::SessionBusy => {
                Self::message(err.to_string())
            }
        }
    }
}

impl From<LoadError> for XyDriverError {
    fn from(err: LoadError) -> Self {
        match err {
            LoadError::Io { .. } | LoadError::Yaml { .. } | LoadError::Deserialize(_) => {
                Self::io(err.to_string())
            }
            LoadError::Template(message) | LoadError::Validation(message) => {
                Self::invalid_input(message)
            }
        }
    }
}

impl From<McpError> for XyDriverError {
    fn from(err: McpError) -> Self {
        match err {
            McpError::Connect(message) | McpError::Call(message) => Self::io(message),
        }
    }
}

impl From<ClipboardError> for XyDriverError {
    fn from(err: ClipboardError) -> Self {
        Self::io(err.0)
    }
}

impl From<crate::agent::capabilities::HookBlockedError> for XyDriverError {
    fn from(err: crate::agent::capabilities::HookBlockedError) -> Self {
        Self::message(err.0)
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
    fn from_opaque_avoids_bare_unavailable_false_positives() {
        let note: XyDriverError = "models list unavailable".into();
        assert_eq!(note.kind(), "Message");

        let busy: XyDriverError = "session resume unavailable while busy".into();
        assert_eq!(busy.kind(), "Message");

        let trust: XyDriverError = "trust option unavailable in this mode".into();
        assert_eq!(trust.kind(), "InvalidInput");
    }

    #[test]
    fn from_opaque_covers_io_and_editor_shapes() {
        let list: XyDriverError = "failed to list sessions: boom".into();
        assert_eq!(list.kind(), "Io");

        let lock: XyDriverError = "create trust lock: boom".into();
        assert_eq!(lock.kind(), "Io");

        let editor: XyDriverError = "set $EDITOR before /editor".into();
        assert_eq!(editor.kind(), "InvalidInput");

        let thinking: XyDriverError = "current model has no thinking levels".into();
        assert_eq!(thinking.kind(), "InvalidInput");
    }

    #[test]
    fn from_opaque_keeps_business_notes_as_message() {
        let empty: XyDriverError = "empty session, nothing to compact".into();
        assert_eq!(empty.kind(), "Message");

        let disabled: XyDriverError = "compaction disabled".into();
        assert_eq!(disabled.kind(), "Message");
    }

    #[test]
    fn from_store_not_found_is_single_layer() {
        let err = XyDriverError::from(XyStoreError::not_found("abc"));
        assert_eq!(err.kind(), "NotFound");
        assert_eq!(err.to_string(), "not found: abc");
    }

    #[test]
    fn from_xy_error_session_flattens_to_store_kind() {
        let err = XyDriverError::from(XyError::from(XyStoreError::not_found("abc")));
        assert_eq!(err.kind(), "NotFound");
        assert_eq!(err.to_string(), "not found: abc");
    }

    #[test]
    fn from_store_validation_keeps_policy_copy() {
        let err = XyDriverError::from(XyStoreError::validation(
            "empty session, nothing to compact",
        ));
        assert_eq!(err.kind(), "Message");
        assert_eq!(err.to_string(), "empty session, nothing to compact");
    }

    #[test]
    fn from_export_and_trust_are_io() {
        let exp = XyDriverError::from(XyExportError::io(
            "write",
            "/tmp/out.html",
            std::io::Error::other("disk full"),
        ));
        assert_eq!(exp.kind(), "Io");
        assert!(exp.to_string().starts_with("io: write"));

        let trust = XyDriverError::from(XyTrustError::Lock);
        assert_eq!(trust.kind(), "Io");
        assert_eq!(trust.to_string(), "io: failed to acquire trust store lock");
    }

    #[test]
    fn from_clipboard_error_is_io() {
        let err = XyDriverError::from(ClipboardError("Clipboard: join error: boom".into()));
        assert_eq!(err.kind(), "Io");
        assert_eq!(err.to_string(), "io: Clipboard: join error: boom");
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
