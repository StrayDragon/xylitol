//! Runtime boundary for permission checks.
//!
//! XyPermission is an advisory in-process permission gate. It politely
//! prevents the ReAct loop from calling a tool that the policy denies, but it
//! is NOT a security boundary: a malicious prompt or a tool like bash can
//! still bypass it at the host level. Real isolation requires OS, container,
//! or VM boundaries. Use this for behavioural guardrails (plan mode,
//! read-only exploration, UX deny-lists), never as a security control.

/// The result of a permission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XyPermissionVerdict {
    /// Access is allowed.
    Allow,
    /// Access is denied with a reason.
    Deny { reason: String },
}

impl XyPermissionVerdict {
    /// Returns `true` if access is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, XyPermissionVerdict::Allow)
    }

    /// Returns the deny reason, or `None` if allowed.
    pub fn deny_reason(&self) -> Option<&str> {
        match self {
            XyPermissionVerdict::Deny { reason } => Some(reason.as_str()),
            XyPermissionVerdict::Allow => None,
        }
    }
}

/// The permission port — checks whether an operation is allowed under the
/// current policy.
///
/// Advisory at the application level: callers (the ReAct loop) are expected to
/// consult this gate before dispatching a tool, but the gate itself does not
/// enforce isolation. Platform-level containment must be provided separately.
pub trait XyPermission: Send + Sync {
    /// Check whether a file read at `path` is allowed.
    fn check_read(&self, path: &str) -> XyPermissionVerdict;
    /// Check whether a file write at `path` is allowed.
    fn check_write(&self, path: &str) -> XyPermissionVerdict;
    /// Check whether a network request to `domain` is allowed.
    fn check_network(&self, domain: &str) -> XyPermissionVerdict;
    /// Check whether spawning a process at `path` is allowed.
    fn check_process(&self, path: &str) -> XyPermissionVerdict;
}
