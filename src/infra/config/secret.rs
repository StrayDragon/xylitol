//! Secret environment file loader.
//!
//! Reads `secret.env` files (dotenv format) from config directories,
//! checks file permissions, and returns key-value pairs for template
//! rendering's `{{ secret.* }}` namespace.

use std::collections::HashMap;
use std::path::Path;

/// Errors from secret loading.
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("permission warning: {path} is readable by group/others. Consider: chmod 600 {path}")]
    Permissions { path: String },
}

/// Result from loading a secret.env file.
pub struct SecretResult {
    pub vars: HashMap<String, String>,
    pub warnings: Vec<SecretError>,
}

/// Load and parse a `secret.env` file at the given path.
///
/// Returns the parsed key-value pairs plus any permission warnings.
/// If the file does not exist, returns an empty map (no error).
pub fn load_secret_env(path: &Path) -> SecretResult {
    let mut warnings = Vec::new();
    let vars = if path.exists() {
        // Check file permissions (Unix only).
        #[cfg(unix)]
        {
            check_file_permissions(path, &mut warnings);
        }

        parse_dotenv_file(path)
    } else {
        HashMap::new()
    };

    SecretResult { vars, warnings }
}

/// Parse a dotenv-format file and return key-value pairs.
fn parse_dotenv_file(path: &Path) -> HashMap<String, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        // Skip empty lines and comments.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Split on first '='.
        if let Some((key, value)) = line.split_once('=') {
            let k = key.trim().to_string();
            let v = value.trim().to_string();
            if !k.is_empty() {
                map.insert(k, v);
            }
        }
    }
    map
}

/// Check file permissions and warn if group/other have read access.
#[cfg(unix)]
fn check_file_permissions(path: &Path, warnings: &mut Vec<SecretError>) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = path.metadata() {
        let mode = meta.permissions().mode();
        // Check group (0o040) or other (0o004) read bit.
        if mode & 0o044 != 0 {
            warnings.push(SecretError::Permissions {
                path: path.to_string_lossy().to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("xylitol_secret_test").join(name);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_parse_dotenv_basic() {
        let dir = test_dir("basic");
        let path = dir.join("secret.env");
        std::fs::write(
            &path,
            b"ANTHROPIC_API_KEY=sk-test\nOPENAI_API_KEY=sk-openai\n",
        )
        .unwrap();

        let result = load_secret_env(&path);
        assert_eq!(result.vars.get("ANTHROPIC_API_KEY").unwrap(), "sk-test");
        assert_eq!(result.vars.get("OPENAI_API_KEY").unwrap(), "sk-openai");
        assert_eq!(result.vars.len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_dotenv_skip_comments_and_empty() {
        let dir = test_dir("comments");
        let path = dir.join("secret.env");
        std::fs::write(&path, b"# this is a comment\n\nKEY=val\nANOTHER=val2\n").unwrap();

        let result = load_secret_env(&path);
        assert!(!result.vars.contains_key(""));
        assert!(!result.vars.contains_key("# this is a comment"));
        assert_eq!(result.vars.get("KEY").unwrap(), "val");
        assert_eq!(result.vars.get("ANOTHER").unwrap(), "val2");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_missing_file_returns_empty() {
        let result = load_secret_env(Path::new("/tmp/nonexistent_secret_test_file.env"));
        assert!(result.vars.is_empty());
        assert!(result.warnings.is_empty());
    }
}
