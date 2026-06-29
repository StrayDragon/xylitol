//! Runtime boundary for sandbox access checks.

/// The result of a sandbox access check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XySandboxVerdict {
    /// Access is allowed.
    Allow,
    /// Access is denied with a reason.
    Deny { reason: String },
}

impl XySandboxVerdict {
    /// Returns `true` if access is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, XySandboxVerdict::Allow)
    }

    /// Returns the deny reason, or `None` if allowed.
    pub fn deny_reason(&self) -> Option<&str> {
        match self {
            XySandboxVerdict::Deny { reason } => Some(reason.as_str()),
            XySandboxVerdict::Allow => None,
        }
    }
}

/// The sandbox engine port — checks whether an operation is allowed under the
/// current sandbox policy. Advisory at the application level; platform backends
/// may enforce at the OS level.
pub trait XySandboxEngine: Send + Sync {
    /// Check whether a file read at `path` is allowed.
    fn check_read(&self, path: &str) -> XySandboxVerdict;
    /// Check whether a file write at `path` is allowed.
    fn check_write(&self, path: &str) -> XySandboxVerdict;
    /// Check whether a network request to `domain` is allowed.
    fn check_network(&self, domain: &str) -> XySandboxVerdict;
    /// Check whether spawning a process at `path` is allowed.
    fn check_process(&self, path: &str) -> XySandboxVerdict;
}
