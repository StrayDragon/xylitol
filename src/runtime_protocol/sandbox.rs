//! Runtime boundary for sandbox access checks.

/// The result of a sandbox access check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxVerdict {
    /// Access is allowed.
    Allow,
    /// Access is denied with a reason.
    Deny { reason: String },
}

impl SandboxVerdict {
    /// Returns `true` if access is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, SandboxVerdict::Allow)
    }

    /// Returns the deny reason, or `None` if allowed.
    pub fn deny_reason(&self) -> Option<&str> {
        match self {
            SandboxVerdict::Deny { reason } => Some(reason.as_str()),
            SandboxVerdict::Allow => None,
        }
    }
}

/// The sandbox engine port — checks whether an operation is allowed under the
/// current sandbox policy. Advisory at the application level; platform backends
/// may enforce at the OS level.
pub trait SandboxEngine: Send + Sync {
    /// Check whether a file read at `path` is allowed.
    fn check_read(&self, path: &str) -> SandboxVerdict;
    /// Check whether a file write at `path` is allowed.
    fn check_write(&self, path: &str) -> SandboxVerdict;
    /// Check whether a network request to `domain` is allowed.
    fn check_network(&self, domain: &str) -> SandboxVerdict;
    /// Check whether spawning a process at `path` is allowed.
    fn check_process(&self, path: &str) -> SandboxVerdict;
}
