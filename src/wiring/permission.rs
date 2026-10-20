//! Map YAML permission config onto runtime backends.

use std::sync::Arc;

use xylitol_plugin_sdk::XyPermission;
use xylitol_runtime::permission::{FilesystemRules, NetworkRules, PermissionRules, ProcessRules};

use super::config::types::PermissionConfig;

pub fn build_permission(config: &PermissionConfig) -> Arc<dyn XyPermission> {
    xylitol_runtime::permission::build_permission(&PermissionRules {
        enabled: config.enabled,
        filesystem: FilesystemRules {
            read_allowed: config.filesystem.read_allowed.clone(),
            write_allowed: config.filesystem.write_allowed.clone(),
            write_denied: config.filesystem.write_denied.clone(),
        },
        network: NetworkRules {
            allowed_domains: config.network.allowed_domains.clone(),
            denied_domains: config.network.denied_domains.clone(),
        },
        process: ProcessRules {
            allowed_paths: config.process.allowed_paths.clone(),
        },
    })
}

pub fn allow_all_permission() -> Arc<dyn XyPermission> {
    xylitol_runtime::permission::allow_all_permission()
}
