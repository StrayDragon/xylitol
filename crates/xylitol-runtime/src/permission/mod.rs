//! Advisory permission backends for the agent runtime.
//!
//! Checks are in-process guardrails, not a security boundary. Real isolation
//! needs OS / container / VM limits.

pub mod policy;

use std::sync::Arc;

use xylitol_plugin_sdk::{XyPermission, XyPermissionVerdict};

/// Filesystem glob rules for [`GlobPolicy`].
#[derive(Clone, Debug, Default)]
pub struct FilesystemRules {
    pub read_allowed: Vec<String>,
    pub write_allowed: Vec<String>,
    pub write_denied: Vec<String>,
}

/// Network domain rules for [`GlobPolicy`].
#[derive(Clone, Debug, Default)]
pub struct NetworkRules {
    pub allowed_domains: Vec<String>,
    pub denied_domains: Vec<String>,
}

/// Process path rules for [`GlobPolicy`].
#[derive(Clone, Debug, Default)]
pub struct ProcessRules {
    pub allowed_paths: Vec<String>,
}

/// Input to [`build_permission`]. Config YAML maps here at the composition root.
#[derive(Clone, Debug, Default)]
pub struct PermissionRules {
    pub enabled: bool,
    pub filesystem: FilesystemRules,
    pub network: NetworkRules,
    pub process: ProcessRules,
}

/// Allow everything. Used when permission checks are off.
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

/// Application-level permission backend using glob-based pattern matching.
#[derive(Clone, Debug)]
pub struct GlobPolicy {
    filesystem: FilesystemRules,
    network: NetworkRules,
    process: ProcessRules,
}

impl GlobPolicy {
    pub fn new(rules: &PermissionRules) -> Self {
        Self {
            filesystem: rules.filesystem.clone(),
            network: rules.network.clone(),
            process: rules.process.clone(),
        }
    }
}

impl XyPermission for GlobPolicy {
    fn check_read(&self, path: &str) -> XyPermissionVerdict {
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
        if policy::path_matches_any(path, &self.filesystem.write_denied) {
            return XyPermissionVerdict::Deny {
                reason: format!("path '{path}' matches permission write_denied"),
            };
        }

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
        if policy::domain_matches_any(domain, &self.network.denied_domains) {
            return XyPermissionVerdict::Deny {
                reason: format!("domain '{domain}' matches permission denied_domains"),
            };
        }

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

/// Build a permission backend from rules.
///
/// Returns [`AllowAllPermission`] when `enabled` is false.
pub fn build_permission(rules: &PermissionRules) -> Arc<dyn XyPermission> {
    if !rules.enabled {
        return Arc::new(AllowAllPermission);
    }
    Arc::new(GlobPolicy::new(rules))
}

/// Allow-all backend (permission checks disabled).
pub fn allow_all_permission() -> Arc<dyn XyPermission> {
    Arc::new(AllowAllPermission)
}
