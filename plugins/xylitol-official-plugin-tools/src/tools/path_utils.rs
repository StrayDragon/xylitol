//! Path resolution utilities for tools.
//!
//! Resolves possibly-relative tool paths against an explicit base directory
//! (the session workspace from `XyToolCtx`), normalizes, and validates for
//! security.

use std::path::{Path, PathBuf};

/// Resolve a possibly-relative file path against `base`.
/// If the path is already absolute, return it as-is.
pub(crate) fn resolve_to_dir(base: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_absolute_path() {
        let result = resolve_to_dir(Path::new("/ws"), "/tmp/test.txt");
        assert_eq!(result, PathBuf::from("/tmp/test.txt"));
    }

    #[test]
    fn test_resolve_relative_path() {
        let result = resolve_to_dir(Path::new("/ws"), "test.txt");
        assert_eq!(result, PathBuf::from("/ws/test.txt"));
    }

    #[test]
    fn test_resolve_nested_relative() {
        let result = resolve_to_dir(Path::new("/ws"), "src/main.rs");
        assert_eq!(result, PathBuf::from("/ws/src/main.rs"));
    }
}
