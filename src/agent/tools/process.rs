//! Shared process helpers for external-process tools (bash, grep, find).
//!
//! Delegates to [`crate::infra::process`] for cross-platform implementation.

/// Kill a process tree by process group ID.
///
/// Uses `kill -9 -<pid>` on Unix, `taskkill /F /T` on Windows.
pub(crate) async fn kill_tree(pid: u32) {
    crate::infra::process::group::kill_process_tree(pid);
}
