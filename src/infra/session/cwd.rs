//! Session CWD validation — checks that the stored session working directory
//! still exists when restoring a session.
//!
//! Aligns with pi's session-cwd.ts.

use std::path::{Path, PathBuf};

use crate::infra::session::types::SessionEntry;

/// Describes a session CWD mismatch issue.
#[derive(Debug, Clone)]
pub struct SessionCwdIssue {
    /// Path to the session file.
    pub session_file: Option<PathBuf>,
    /// The stored CWD that no longer exists.
    pub session_cwd: PathBuf,
    /// The fallback CWD to use instead.
    pub fallback_cwd: PathBuf,
}

/// Error thrown when a session's stored CWD no longer exists.
#[derive(Debug)]
pub struct MissingSessionCwdError {
    pub issue: SessionCwdIssue,
}

impl std::fmt::Display for MissingSessionCwdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_missing_session_cwd_error(&self.issue))
    }
}

impl std::error::Error for MissingSessionCwdError {}

/// Check if the session's stored CWD is missing and return details.
///
/// Returns `None` if:
/// - No session is loaded (in-memory session)
/// - The stored CWD exists on disk
pub fn get_missing_session_cwd_issue(
    entries: &[SessionEntry],
    session_file: Option<PathBuf>,
    fallback_cwd: &Path,
) -> Option<SessionCwdIssue> {
    let header = entries.iter().find_map(|e| {
        if let SessionEntry::Header(h) = e {
            Some(h)
        } else {
            None
        }
    })?;

    let session_cwd = if header.cwd.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(&header.cwd)
    };

    if session_cwd.exists() {
        return None;
    }

    Some(SessionCwdIssue {
        session_file,
        session_cwd,
        fallback_cwd: fallback_cwd.to_path_buf(),
    })
}

/// Assert that the session's stored CWD exists.
pub fn assert_session_cwd_exists(
    entries: &[SessionEntry],
    session_file: Option<PathBuf>,
    fallback_cwd: &Path,
) -> Result<(), MissingSessionCwdError> {
    if let Some(issue) = get_missing_session_cwd_issue(entries, session_file, fallback_cwd) {
        Err(MissingSessionCwdError { issue })
    } else {
        Ok(())
    }
}

/// Format a user-facing error message for a missing session CWD.
pub fn format_missing_session_cwd_error(issue: &SessionCwdIssue) -> String {
    let session_info = issue
        .session_file
        .as_ref()
        .map(|p| format!("\nSession file: {}", p.display()))
        .unwrap_or_default();
    format!(
        "Stored session working directory does not exist: {}{}\nCurrent working directory: {}",
        issue.session_cwd.display(),
        session_info,
        issue.fallback_cwd.display()
    )
}

/// Format a short prompt message for inline display.
pub fn format_missing_session_cwd_prompt(issue: &SessionCwdIssue) -> String {
    format!(
        "cwd from session file does not exist\n{}\n\ncontinue in current cwd\n{}",
        issue.session_cwd.display(),
        issue.fallback_cwd.display()
    )
}
