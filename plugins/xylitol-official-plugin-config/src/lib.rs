pub mod error;
pub mod loader;
pub mod paths;
pub mod plugin;
pub mod secret_env;
pub mod template;
pub mod types;
pub mod validate;

// ConfigValueResolver（r1821-1823）：spec 要求提供的配置值解析能力
// （literal / $VAR / ${VAR:-default} / `!command` 带预算执行与进程生命周期缓存）。
// 产品接入（r1912）：`load_from_paths` 装配 models.models 条目值时经
// `resolver::resolve_from_env` 展开表达式（见 loader.rs）。
pub mod resolver;
pub mod settings;

use std::sync::Arc;

use xylitol_plugin_sdk::XyPermission;
use xylitol_runtime::permission::{FilesystemRules, NetworkRules, PermissionRules, ProcessRules};

use types::PermissionConfig;

impl PermissionConfig {
    /// Map YAML permission onto the runtime backend.
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
