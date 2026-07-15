//! Permission policy backends for the agent runtime.
//!
//! Provides the [`XyPermission`] trait (defined in [`crate::runtime_protocol::permission`])
//! and concrete backends that enforce application-level path/domain matching.
//! Platform-specific backends (Landlock, macOS sandbox) are added as separate
//! modules.
//!
//! # Advisory only
//!
//! The checks in this module are advisory in-process guardrails. They are NOT
//! a security boundary and do not prevent host-level access. Real isolation
//! requires OS, container, or VM boundaries.

pub mod policy;

use std::sync::Arc;

use super::config::types::{
    PermissionBackend, PermissionConfig, PermissionFilesystemConfig, PermissionNetworkConfig,
    PermissionProcessConfig,
};

// XyPermissionVerdict + XyPermission trait live in `runtime_protocol::permission`.
// Concrete backends below (AllowAllPermission, GlobPolicy, platform) implement
// the port; re-exported here for existing `crate::infra::permission::*` references.
pub use crate::runtime_protocol::{XyPermission, XyPermissionVerdict};

/// A permission backend that allows everything. Used when permission checks are
/// disabled.
#[derive(Clone, Debug)]
pub struct AllowAllPermission;

impl XyPermission for AllowAllPermission {
    fn check_read(&self, _path: &str) -> XyPermissionVerdict {
        XyPermissionVerdict::Allow
    }

    fn check_write(&self, _path: &str) -> XyPermissionVerdict {
        XyPermissionVerdict::Allow
    }

    fn check_network(&self, _domain: &str) -> XyPermissionVerdict {
        XyPermissionVerdict::Allow
    }

    fn check_process(&self, _path: &str) -> XyPermissionVerdict {
        XyPermissionVerdict::Allow
    }
}

// ── GlobPolicy ──────────────────────────────────────────────────────

/// Application-level permission backend using glob-based pattern matching.
///
/// Enforces:
/// - Filesystem: `read_allowed` + `write_allowed` / `write_denied` with default-deny
/// - Network: `allowed_domains` / `denied_domains` with default-deny
/// - Process: `allowed_paths` with default-deny
#[derive(Clone, Debug)]
pub struct GlobPolicy {
    filesystem: PermissionFilesystemConfig,
    network: PermissionNetworkConfig,
    process: PermissionProcessConfig,
}

impl GlobPolicy {
    pub fn new(config: &PermissionConfig) -> Self {
        Self {
            filesystem: config.filesystem.clone(),
            network: config.network.clone(),
            process: config.process.clone(),
        }
    }
}

impl XyPermission for GlobPolicy {
    fn check_read(&self, path: &str) -> XyPermissionVerdict {
        // When read_allowed is non-empty, default-deny for unlisted paths.
        if !self.filesystem.read_allowed.is_empty()
            && !policy::path_matches_any(path, &self.filesystem.read_allowed)
        {
            return XyPermissionVerdict::Deny {
                reason: format!("path '{path}' is not in permission read_allowed list"),
            };
        }
        XyPermissionVerdict::Allow
    }

    fn check_write(&self, path: &str) -> XyPermissionVerdict {
        // Check write_denied first (most specific deny wins).
        if policy::path_matches_any(path, &self.filesystem.write_denied) {
            return XyPermissionVerdict::Deny {
                reason: format!("path '{path}' matches permission write_denied"),
            };
        }

        // When write_allowed is non-empty, default-deny for unlisted paths.
        if !self.filesystem.write_allowed.is_empty()
            && !policy::path_matches_any(path, &self.filesystem.write_allowed)
        {
            return XyPermissionVerdict::Deny {
                reason: format!("path '{path}' is not in permission write_allowed list"),
            };
        }

        XyPermissionVerdict::Allow
    }

    fn check_network(&self, domain: &str) -> XyPermissionVerdict {
        // Check denied_domains first.
        if policy::domain_matches_any(domain, &self.network.denied_domains) {
            return XyPermissionVerdict::Deny {
                reason: format!("domain '{domain}' matches permission denied_domains"),
            };
        }

        // When allowed_domains is non-empty, default-deny for unlisted.
        if !self.network.allowed_domains.is_empty()
            && !policy::domain_matches_any(domain, &self.network.allowed_domains)
        {
            return XyPermissionVerdict::Deny {
                reason: format!("domain '{domain}' is not in permission allowed_domains list"),
            };
        }

        XyPermissionVerdict::Allow
    }

    fn check_process(&self, path: &str) -> XyPermissionVerdict {
        if !self.process.allowed_paths.is_empty()
            && !policy::path_matches_any(path, &self.process.allowed_paths)
        {
            return XyPermissionVerdict::Deny {
                reason: format!("process path '{path}' is not in permission process allowed_paths"),
            };
        }
        XyPermissionVerdict::Allow
    }
}

// ── Factory ─────────────────────────────────────────────────────────

/// Build a permission backend from config.
///
/// Returns `AllowAllPermission` when permission is disabled or the feature is not enabled.
pub fn build_permission(config: &PermissionConfig) -> Arc<dyn XyPermission> {
    if !config.enabled {
        return Arc::new(AllowAllPermission);
    }

    match config.backend {
        PermissionBackend::Glob => Arc::new(GlobPolicy::new(config)),
        PermissionBackend::Landlock => {
            // Landlock backend is not yet implemented — fall back gracefully.
            log::warn!("Landlock permission backend not yet implemented, using GlobPolicy");
            Arc::new(GlobPolicy::new(config))
        }
        PermissionBackend::MacOs => {
            log::warn!("macOS permission backend not yet implemented, using GlobPolicy");
            Arc::new(GlobPolicy::new(config))
        }
    }
}

/// Build an allow-all backend (permission checks disabled).
pub fn allow_all_permission() -> Arc<dyn XyPermission> {
    Arc::new(AllowAllPermission)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> PermissionConfig {
        PermissionConfig {
            enabled: true,
            backend: PermissionBackend::Glob,
            filesystem: PermissionFilesystemConfig {
                read_allowed: vec!["/home/user/project/**".into()],
                write_allowed: vec!["/home/user/project/**".into()],
                write_denied: vec!["**/.env".into(), "**/*.pem".into()],
            },
            network: PermissionNetworkConfig {
                allowed_domains: vec!["github.com".into(), "*.github.com".into()],
                denied_domains: vec!["evil.com".into()],
            },
            process: PermissionProcessConfig {
                allowed_paths: vec![],
            },
        }
    }

    #[test]
    fn test_glob_policy_allows_project_read() {
        let engine = GlobPolicy::new(&make_config());
        assert!(
            engine
                .check_read("/home/user/project/src/main.rs")
                .is_allowed()
        );
    }

    #[test]
    fn test_glob_policy_denies_outside_read() {
        let engine = GlobPolicy::new(&make_config());
        let verdict = engine.check_read("/etc/passwd");
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().unwrap().contains("read_allowed"));
    }

    #[test]
    fn test_glob_policy_denies_env_write() {
        let engine = GlobPolicy::new(&make_config());
        let verdict = engine.check_write("/home/user/project/.env");
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().unwrap().contains("write_denied"));
    }

    #[test]
    fn test_glob_policy_allows_project_write() {
        let engine = GlobPolicy::new(&make_config());
        assert!(
            engine
                .check_write("/home/user/project/src/lib.rs")
                .is_allowed()
        );
    }

    #[test]
    fn test_glob_policy_denies_evil_domain() {
        let engine = GlobPolicy::new(&make_config());
        let verdict = engine.check_network("evil.com");
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().unwrap().contains("denied_domains"));
    }

    #[test]
    fn test_glob_policy_allows_github_domain() {
        let engine = GlobPolicy::new(&make_config());
        assert!(engine.check_network("github.com").is_allowed());
        assert!(engine.check_network("api.github.com").is_allowed());
    }

    #[test]
    fn test_glob_policy_denies_unlisted_domain() {
        let engine = GlobPolicy::new(&make_config());
        let verdict = engine.check_network("example.com");
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().unwrap().contains("allowed_domains"));
    }

    #[test]
    fn test_allow_all_allows_everything() {
        let engine = AllowAllPermission;
        assert!(engine.check_read("/etc/shadow").is_allowed());
        assert!(engine.check_write("/etc/passwd").is_allowed());
        assert!(engine.check_network("evil.com").is_allowed());
        assert!(engine.check_process("/bin/rm").is_allowed());
    }

    #[test]
    fn test_build_permission_disabled_returns_allow_all() {
        let mut config = make_config();
        config.enabled = false;
        let engine = build_permission(&config);
        assert!(engine.check_read("/etc/passwd").is_allowed());
    }

    #[test]
    fn test_build_permission_enabled_returns_glob_policy() {
        let config = make_config();
        let engine = build_permission(&config);
        assert!(!engine.check_read("/etc/passwd").is_allowed());
    }
}
