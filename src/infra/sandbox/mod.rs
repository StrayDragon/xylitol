//! Sandbox execution isolation — tools/processes run in a policy-constrained environment.
//!
//! Provides the [`SandboxEngine`] trait and a [`FallbackBackend`] that enforces
//! application-level path/domain matching. Platform-specific backends (Landlock,
//! macOS sandbox) are added as separate modules.

pub mod policy;

use std::sync::Arc;

use super::config::types::{
    SandboxBackend, SandboxConfig, SandboxFilesystemConfig, SandboxNetworkConfig,
    SandboxProcessConfig,
};

// SandboxVerdict + SandboxEngine trait relocated to `core::ports` (shared port
// vocabulary). Concrete backends below (NoopEngine, FallbackBackend, platform)
// implement the port; re-exported here for existing `crate::infra::sandbox::*`
// references.
pub use crate::core::ports::{SandboxEngine, SandboxVerdict};

/// A no-op engine that allows everything. Used when sandbox is disabled.
#[derive(Clone, Debug)]
pub struct NoopEngine;

impl SandboxEngine for NoopEngine {
    fn check_read(&self, _path: &str) -> SandboxVerdict {
        SandboxVerdict::Allow
    }

    fn check_write(&self, _path: &str) -> SandboxVerdict {
        SandboxVerdict::Allow
    }

    fn check_network(&self, _domain: &str) -> SandboxVerdict {
        SandboxVerdict::Allow
    }

    fn check_process(&self, _path: &str) -> SandboxVerdict {
        SandboxVerdict::Allow
    }
}

// ── FallbackBackend ─────────────────────────────────────────────────

/// Application-level sandbox engine using glob-based pattern matching.
///
/// Enforces:
/// - Filesystem: `read_allowed` + `write_allowed` / `write_denied` with default-deny
/// - Network: `allowed_domains` / `denied_domains` with default-deny
/// - Process: `allowed_paths` with default-deny
#[derive(Clone, Debug)]
pub struct FallbackBackend {
    filesystem: SandboxFilesystemConfig,
    network: SandboxNetworkConfig,
    process: SandboxProcessConfig,
}

impl FallbackBackend {
    pub fn new(config: &SandboxConfig) -> Self {
        Self {
            filesystem: config.filesystem.clone(),
            network: config.network.clone(),
            process: config.process.clone(),
        }
    }
}

impl SandboxEngine for FallbackBackend {
    fn check_read(&self, path: &str) -> SandboxVerdict {
        // When read_allowed is non-empty, default-deny for unlisted paths.
        if !self.filesystem.read_allowed.is_empty()
            && !policy::path_matches_any(path, &self.filesystem.read_allowed)
        {
            return SandboxVerdict::Deny {
                reason: format!("path '{path}' is not in sandbox read_allowed list"),
            };
        }
        SandboxVerdict::Allow
    }

    fn check_write(&self, path: &str) -> SandboxVerdict {
        // Check write_denied first (most specific deny wins).
        if policy::path_matches_any(path, &self.filesystem.write_denied) {
            return SandboxVerdict::Deny {
                reason: format!("path '{path}' matches sandbox write_denied"),
            };
        }

        // When write_allowed is non-empty, default-deny for unlisted paths.
        if !self.filesystem.write_allowed.is_empty()
            && !policy::path_matches_any(path, &self.filesystem.write_allowed)
        {
            return SandboxVerdict::Deny {
                reason: format!("path '{path}' is not in sandbox write_allowed list"),
            };
        }

        SandboxVerdict::Allow
    }

    fn check_network(&self, domain: &str) -> SandboxVerdict {
        // Check denied_domains first.
        if policy::domain_matches_any(domain, &self.network.denied_domains) {
            return SandboxVerdict::Deny {
                reason: format!("domain '{domain}' matches sandbox denied_domains"),
            };
        }

        // When allowed_domains is non-empty, default-deny for unlisted.
        if !self.network.allowed_domains.is_empty()
            && !policy::domain_matches_any(domain, &self.network.allowed_domains)
        {
            return SandboxVerdict::Deny {
                reason: format!("domain '{domain}' is not in sandbox allowed_domains list"),
            };
        }

        SandboxVerdict::Allow
    }

    fn check_process(&self, path: &str) -> SandboxVerdict {
        if !self.process.allowed_paths.is_empty()
            && !policy::path_matches_any(path, &self.process.allowed_paths)
        {
            return SandboxVerdict::Deny {
                reason: format!("process path '{path}' is not in sandbox process allowed_paths"),
            };
        }
        SandboxVerdict::Allow
    }
}

// ── Factory ─────────────────────────────────────────────────────────

/// Build a sandbox engine from config.
///
/// Returns `NoopEngine` when sandbox is disabled or the feature is not enabled.
pub fn build_engine(config: &SandboxConfig) -> Arc<dyn SandboxEngine> {
    if !config.enabled {
        return Arc::new(NoopEngine);
    }

    match config.backend {
        SandboxBackend::Fallback => Arc::new(FallbackBackend::new(config)),
        SandboxBackend::Landlock => {
            // Landlock backend is not yet implemented — fall back gracefully.
            tracing::warn!("Landlock sandbox backend not yet implemented, using FallbackBackend");
            Arc::new(FallbackBackend::new(config))
        }
        SandboxBackend::MacOs => {
            tracing::warn!("macOS sandbox backend not yet implemented, using FallbackBackend");
            Arc::new(FallbackBackend::new(config))
        }
    }
}

/// Build a no-op engine (sandbox disabled).
pub fn noop_engine() -> Arc<dyn SandboxEngine> {
    Arc::new(NoopEngine)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> SandboxConfig {
        SandboxConfig {
            enabled: true,
            backend: SandboxBackend::Fallback,
            filesystem: SandboxFilesystemConfig {
                read_allowed: vec!["/home/user/project/**".into()],
                write_allowed: vec!["/home/user/project/**".into()],
                write_denied: vec!["**/.env".into(), "**/*.pem".into()],
            },
            network: SandboxNetworkConfig {
                allowed_domains: vec!["github.com".into(), "*.github.com".into()],
                denied_domains: vec!["evil.com".into()],
            },
            process: SandboxProcessConfig {
                allowed_paths: vec![],
            },
        }
    }

    #[test]
    fn test_fallback_allows_project_read() {
        let engine = FallbackBackend::new(&make_config());
        assert!(
            engine
                .check_read("/home/user/project/src/main.rs")
                .is_allowed()
        );
    }

    #[test]
    fn test_fallback_denies_outside_read() {
        let engine = FallbackBackend::new(&make_config());
        let verdict = engine.check_read("/etc/passwd");
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().unwrap().contains("read_allowed"));
    }

    #[test]
    fn test_fallback_denies_env_write() {
        let engine = FallbackBackend::new(&make_config());
        let verdict = engine.check_write("/home/user/project/.env");
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().unwrap().contains("write_denied"));
    }

    #[test]
    fn test_fallback_allows_project_write() {
        let engine = FallbackBackend::new(&make_config());
        assert!(
            engine
                .check_write("/home/user/project/src/lib.rs")
                .is_allowed()
        );
    }

    #[test]
    fn test_fallback_denies_evil_domain() {
        let engine = FallbackBackend::new(&make_config());
        let verdict = engine.check_network("evil.com");
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().unwrap().contains("denied_domains"));
    }

    #[test]
    fn test_fallback_allows_github_domain() {
        let engine = FallbackBackend::new(&make_config());
        assert!(engine.check_network("github.com").is_allowed());
        assert!(engine.check_network("api.github.com").is_allowed());
    }

    #[test]
    fn test_fallback_denies_unlisted_domain() {
        let engine = FallbackBackend::new(&make_config());
        let verdict = engine.check_network("example.com");
        assert!(!verdict.is_allowed());
        assert!(verdict.deny_reason().unwrap().contains("allowed_domains"));
    }

    #[test]
    fn test_noop_allows_everything() {
        let engine = NoopEngine;
        assert!(engine.check_read("/etc/shadow").is_allowed());
        assert!(engine.check_write("/etc/passwd").is_allowed());
        assert!(engine.check_network("evil.com").is_allowed());
        assert!(engine.check_process("/bin/rm").is_allowed());
    }

    #[test]
    fn test_build_engine_disabled_returns_noop() {
        let mut config = make_config();
        config.enabled = false;
        let engine = build_engine(&config);
        assert!(engine.check_read("/etc/passwd").is_allowed());
    }

    #[test]
    fn test_build_engine_enabled_returns_fallback() {
        let config = make_config();
        let engine = build_engine(&config);
        assert!(!engine.check_read("/etc/passwd").is_allowed());
    }
}
