//! Path resolution utilities for tools.
//!
//! Resolves relative paths to the current working directory,
//! normalizes, and validates for security.

use std::path::{Path, PathBuf};

/// Resolve a possibly-relative file path to an absolute path under the current working directory.
/// If the path is already absolute, return it as-is.
pub fn resolve_to_cwd(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(p)
    }
}

/// Check if a resolved path is within the given root (prevents directory traversal).
pub fn is_within_root(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

/// Normalize a path by resolving `..` and `.` components.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            c => normalized.push(c),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_absolute_path() {
        let result = resolve_to_cwd("/tmp/test.txt");
        assert_eq!(result, PathBuf::from("/tmp/test.txt"));
    }

    #[test]
    fn test_resolve_relative_path() {
        let result = resolve_to_cwd("test.txt");
        assert!(result.is_absolute());
        assert!(result.ends_with("test.txt"));
    }

    #[test]
    fn test_is_within_root() {
        let root = PathBuf::from("/home/user/project");
        assert!(is_within_root(&root.join("src/main.rs"), &root));
        assert!(!is_within_root(Path::new("/etc/passwd"), &root));
    }

    #[test]
    fn test_normalize_path_dot_dot() {
        let result = normalize_path(Path::new("/foo/bar/../baz"));
        assert_eq!(result, PathBuf::from("/foo/baz"));
    }

    #[test]
    fn test_normalize_path_dot() {
        let result = normalize_path(Path::new("/foo/./bar"));
        assert_eq!(result, PathBuf::from("/foo/bar"));
    }
}
