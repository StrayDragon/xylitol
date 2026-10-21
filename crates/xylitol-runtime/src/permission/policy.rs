pub fn path_matches_any(path: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    patterns.iter().any(|p| path_matches(path, p))
}

fn path_matches(path: &str, pattern: &str) -> bool {
    match glob::Pattern::new(pattern) {
        Ok(p) => p.matches(path),
        Err(_) => path.contains(pattern),
    }
}

pub fn domain_matches_any(domain: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    patterns.iter().any(|p| domain_matches(domain, p))
}

fn domain_matches(domain: &str, pattern: &str) -> bool {
    let domain = domain.trim().to_lowercase();
    let pattern = pattern.trim().to_lowercase();

    if pattern == "*" {
        return true;
    }

    if domain == pattern {
        return true;
    }

    if let Some(rest) = pattern.strip_prefix("*.") {
        if domain.ends_with(rest) && domain.len() > rest.len() {
            let before = &domain[..domain.len() - rest.len()];
            return before.ends_with('.');
        }
        return false;
    }

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

    #[test]
    fn test_path_exact_match() {
        assert!(path_matches("/home/user/file.txt", "/home/user/file.txt"));
    }

    #[test]
    fn test_path_glob_star() {
        assert!(path_matches("/home/user/file.txt", "/home/user/*.txt"));
    }

    #[test]
    fn test_path_glob_double_star() {
        assert!(path_matches("/home/user/src/main.rs", "/home/user/**"));
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
        assert!(path_matches("/project/.env", "**/.env"));
        assert!(path_matches("/project/sub/.env", "**/.env"));
    }

    #[test]
    fn test_path_pem_pattern() {
        assert!(path_matches("/project/secret.pem", "**/*.pem"));
        assert!(path_matches("/project/keys/rsa.pem", "**/*.pem"));
    }

    #[test]
    fn test_domain_exact_match() {
        assert!(domain_matches("example.com", "example.com"));
    }

    #[test]
    fn test_domain_wildcard_subdomain() {
        assert!(domain_matches("api.github.com", "*.github.com"));
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
        assert!(domain_matches("sub.example.com", "*.example.com"));
    }
}
