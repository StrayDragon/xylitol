pub mod assembly;
pub mod bootstrap;
pub mod composition;
pub mod mcp_spec;

pub mod admit;
pub mod attach;
pub mod boot;
pub mod builtins;
pub mod core;
pub mod dispatch;
pub mod driver;
pub mod event;
pub mod export;
pub mod external_plugin;
pub mod product_commands;
pub mod provider;
pub mod subagent;
pub mod surfaces;

#[cfg(feature = "server")]
pub mod occupancy;

#[cfg(test)]
mod contract;

pub use xylitol_app_print as app_print;
pub use xylitol_official_plugin_config as config;
pub use xylitol_official_plugin_config::settings;
pub use xylitol_official_plugin_obs as obs;
pub use xylitol_official_plugin_obs as timing;

pub use boot::{BootOptions, Kernel, Profile, boot, boot_ctx};
