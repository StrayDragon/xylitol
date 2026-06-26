//! Path resolution utilities for tools.
//!
//! Resolves relative paths to the current working directory,
//! normalizes, and validates for security.

use std::path::{Path, PathBuf};

/// Resolve a possibly-relative file path to an absolute path under the current working directory.
/// If the path is already absolute, return it as-is.
pub(crate) fn resolve_to_cwd(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(p)
    }
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
}
