//! Git branch detection.
//!
//! Reads HEAD reference to determine current branch.

use std::path::Path;

/// Read the current branch from a git HEAD file.
///
/// Returns `Some(name)` for a named branch, `None` for detached HEAD.
pub fn get_current_branch(head_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(head_path).ok()?;
    let trimmed = content.trim();
    if let Some(ref_name) = trimmed.strip_prefix("ref: refs/heads/") {
        Some(ref_name.trim().to_string())
    } else {
        // Detached HEAD — contains a commit hash
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_current_branch_named() {
        let dir = tempfile::tempdir().unwrap();
        let head_path = dir.path().join("HEAD");
        fs::write(&head_path, "ref: refs/heads/main\n").unwrap();
        assert_eq!(get_current_branch(&head_path).unwrap(), "main");
    }

    #[test]
    fn test_get_current_branch_detached() {
        let dir = tempfile::tempdir().unwrap();
        let head_path = dir.path().join("HEAD");
        fs::write(&head_path, "abc123def456\n").unwrap();
        assert!(get_current_branch(&head_path).is_none());
    }

    #[test]
    fn test_get_current_branch_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let head_path = dir.path().join("HEAD");
        assert!(get_current_branch(&head_path).is_none());
    }
}
