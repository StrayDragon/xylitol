//! Git URL parsing.
//!
//! Handles SCP-like (`git@host:path`), HTTPS, SSH, and git:// URLs.

/// Parsed git source information.
#[derive(Debug, Clone, PartialEq)]
pub struct GitSource {
    /// Clone URL (always valid for git clone).
    pub repo: String,
    /// Host domain (e.g., "github.com").
    pub host: String,
    /// Repository path (e.g., "user/repo").
    pub path: String,
    /// Git ref (branch, tag, commit) if specified.
    pub r#ref: Option<String>,
    /// True if ref was specified (pinned).
    pub pinned: bool,
}

/// Parse a git URL into its components.
///
/// Supports:
/// - SCP-like: `git@github.com:user/repo.git`
/// - HTTPS: `https://github.com/user/repo.git`
/// - SSH: `ssh://git@github.com/user/repo.git`
/// - Git: `git://github.com/user/repo.git`
pub fn parse_git_url(url: &str) -> Option<GitSource> {
    let trimmed = url.trim();

    // Extract optional ref fragment (#ref)
    let (base_url, ref_str) = if let Some(idx) = trimmed.rfind('#') {
        (&trimmed[..idx], Some(trimmed[idx + 1..].to_string()))
    } else {
        (trimmed, None)
    };

    let pinned = ref_str.is_some();

    if let Some(scp) = parse_scp_like(base_url) {
        return Some(GitSource {
            repo: scp.0,
            host: scp.1,
            path: scp.2,
            r#ref: ref_str,
            pinned,
        });
    }

    if let Some(protocol) = parse_protocol_url(base_url) {
        return Some(GitSource {
            repo: protocol.0,
            host: protocol.1,
            path: protocol.2,
            r#ref: ref_str,
            pinned,
        });
    }

    None
}

/// Parse SCP-like URLs: `git@host:path`
fn parse_scp_like(url: &str) -> Option<(String, String, String)> {
    let url = url.strip_prefix("git@")?;
    let colon_idx = url.find(':')?;
    let host = url[..colon_idx].to_string();
    let path = url[colon_idx + 1..].trim_end_matches(".git").to_string();
    let repo = format!("git@{}:{}.git", host, path);
    Some((repo, host, path))
}

/// Parse protocol URLs: `https://`, `ssh://`, `git://`
fn parse_protocol_url(url: &str) -> Option<(String, String, String)> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_string();
    let path = parsed
        .path()
        .trim_start_matches('/')
        .trim_end_matches(".git")
        .to_string();
    if path.is_empty() || !path.contains('/') {
        return None;
    }
    Some((url.to_string(), host, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scp_url() {
        let result = parse_git_url("git@github.com:user/repo.git").unwrap();
        assert_eq!(result.host, "github.com");
        assert_eq!(result.path, "user/repo");
        assert!(!result.pinned);
    }

    #[test]
    fn test_parse_https_url() {
        let result = parse_git_url("https://github.com/user/repo.git").unwrap();
        assert_eq!(result.host, "github.com");
        assert_eq!(result.path, "user/repo");
    }

    #[test]
    fn test_parse_url_with_ref() {
        let result = parse_git_url("git@github.com:user/repo.git#v1.0").unwrap();
        assert_eq!(result.r#ref.unwrap(), "v1.0");
        assert!(result.pinned);
    }

    #[test]
    fn test_parse_invalid_url() {
        assert!(parse_git_url("not-a-url").is_none());
    }
}
