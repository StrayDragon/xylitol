//! Policy matching utilities for sandbox access control.
//!
//! Provides glob-based path matching and wildcard domain matching used by
//! [`FallbackBackend`](super::FallbackBackend).

/// Check whether `path` matches any of the given glob patterns.
///
/// Uses the glob crate's pattern syntax. Patterns are matched against the
/// full absolute path (not basename). Returns `true` if any pattern matches.
pub fn path_matches_any(path: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    patterns.iter().any(|p| path_matches(path, p))
}

/// Check whether a single path matches a glob pattern.
///
/// Uses the `glob` crate's `Pattern` which supports `**` for recursive
/// directory matching (e.g., `**/.env` matches `foo/.env` and `bar/baz/.env`).
fn path_matches(path: &str, pattern: &str) -> bool {
    match glob::Pattern::new(pattern) {
        Ok(p) => p.matches(path),
        Err(_) => {
            // Invalid pattern: fall back to substring match
            path.contains(pattern)
        }
    }
}

/// Check whether `domain` matches any of the given domain patterns.
///
/// Supports the following pattern formats:
/// - `example.com` — exact match
/// - `*.example.com` — matches any subdomain of example.com
/// - `example.*` — matches example.com, example.org, etc.
/// - `*` — matches everything
pub fn domain_matches_any(domain: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    patterns.iter().any(|p| domain_matches(domain, p))
}

/// Check whether a single domain matches a domain pattern.
fn domain_matches(domain: &str, pattern: &str) -> bool {
    let domain = domain.trim().to_lowercase();
    let pattern = pattern.trim().to_lowercase();

    // Wildcard: matches everything
    if pattern == "*" {
        return true;
    }

    // Exact match
    if domain == pattern {
        return true;
    }

    // `*.example.com` — match any subdomain
    if let Some(rest) = pattern.strip_prefix("*.") {
        if domain.ends_with(rest) && domain.len() > rest.len() {
            let before = &domain[..domain.len() - rest.len()];
            return before.ends_with('.');
        }
        return false;
    }

    // `example.*` — match any TLD
    if let Some(prefix) = pattern.strip_suffix(".*") {
        if domain.starts_with(prefix) && domain.len() > prefix.len() {
            let after = &domain[prefix.len()..];
            return after.starts_with('.');
        }
        return false;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Path matching ───────────────────────────────────────────

    #[test]
    fn test_path_exact_match() {
        assert!(path_matches("/home/user/file.txt", "/home/user/file.txt"));
    }

    #[test]
    fn test_path_glob_star() {
        assert!(path_matches(
            "/home/user/file.txt",
            "/home/user/*.txt"
        ));
    }

    #[test]
    fn test_path_glob_double_star() {
        assert!(path_matches(
            "/home/user/src/main.rs",
            "/home/user/**"
        ));
    }

    #[test]
    fn test_path_glob_double_star_deep() {
        assert!(path_matches(
            "/home/user/src/deep/nested/file.rs",
            "/home/user/**"
        ));
    }

    #[test]
    fn test_path_no_match() {
        assert!(!path_matches("/etc/passwd", "/home/**"));
    }

    #[test]
    fn test_path_multiple_patterns() {
        let patterns = vec!["/home/**".into(), "/tmp/**".into()];
        assert!(path_matches_any("/home/user/x.txt", &patterns));
        assert!(path_matches_any("/tmp/x.txt", &patterns));
        assert!(!path_matches_any("/etc/x.txt", &patterns));
    }

    #[test]
    fn test_path_empty_patterns() {
        assert!(!path_matches_any("/any/path", &[]));
    }

    #[test]
    fn test_path_env_file_pattern() {
        // `**/.env` should match `.env` at any depth
        assert!(path_matches("/project/.env", "**/.env"));
        assert!(path_matches("/project/sub/.env", "**/.env"));
    }

    #[test]
    fn test_path_pem_pattern() {
        assert!(path_matches("/project/secret.pem", "**/*.pem"));
        assert!(path_matches("/project/keys/rsa.pem", "**/*.pem"));
    }

    // ── Domain matching ─────────────────────────────────────────

    #[test]
    fn test_domain_exact_match() {
        assert!(domain_matches("example.com", "example.com"));
    }

    #[test]
    fn test_domain_wildcard_subdomain() {
        assert!(domain_matches("api.github.com", "*.github.com"));
        // `*.github.com` matches subdomains only, not the root domain itself
        assert!(!domain_matches("github.com", "*.github.com"));
    }

    #[test]
    fn test_domain_wildcard_tld() {
        assert!(domain_matches("example.com", "example.*"));
        assert!(domain_matches("example.org", "example.*"));
        assert!(!domain_matches("exampleex.com", "example.*"));
    }

    #[test]
    fn test_domain_universal_wildcard() {
        assert!(domain_matches("anything.here", "*"));
    }

    #[test]
    fn test_domain_no_match() {
        assert!(!domain_matches("evil.com", "good.com"));
    }

    #[test]
    fn test_domain_multiple_patterns() {
        let patterns = vec!["github.com".into(), "gitlab.com".into()];
        assert!(domain_matches_any("github.com", &patterns));
        assert!(domain_matches_any("gitlab.com", &patterns));
        assert!(!domain_matches_any("bitbucket.org", &patterns));
    }

    #[test]
    fn test_domain_case_insensitive() {
        assert!(domain_matches("Evil.COM", "evil.com"));
        assert!(domain_matches("evil.com", "Evil.COM"));
    }

    #[test]
    fn test_domain_subdomain_no_match_wrong_level() {
        // `*.example.com` should match sub.example.com but not example.com
        // Actually checking pi's behavior: `*.example.com` matches `sub.example.com`
        assert!(domain_matches("sub.example.com", "*.example.com"));
    }
}
