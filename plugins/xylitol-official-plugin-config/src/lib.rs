pub mod error;
pub mod loader;
pub mod paths;
pub mod plugin;
pub mod secret_env;
pub mod template;
pub mod types;
pub mod validate;

pub mod resolver;
pub mod settings;

use std::sync::Arc;

use xylitol_plugin_sdk::XyPermission;
use xylitol_runtime::permission::{FilesystemRules, NetworkRules, PermissionRules, ProcessRules};

use types::PermissionConfig;

impl PermissionConfig {
    pub fn to_permission(&self) -> Arc<dyn XyPermission> {
        xylitol_runtime::permission::build_permission(&PermissionRules {
            enabled: self.enabled,
            filesystem: FilesystemRules {
                read_allowed: self.filesystem.read_allowed.clone(),
                write_allowed: self.filesystem.write_allowed.clone(),
                write_denied: self.filesystem.write_denied.clone(),
            },
            network: NetworkRules {
                allowed_domains: self.network.allowed_domains.clone(),
                denied_domains: self.network.denied_domains.clone(),
            },
            process: ProcessRules {
                allowed_paths: self.process.allowed_paths.clone(),
            },
        })
    }
}
