use std::path::PathBuf;

use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::loader::load_app_config_detailed;
use crate::types::AppConfig;

pub struct ConfigService {
    pub app: AppConfig,
    pub agent_dir: PathBuf,
    pub cwd: PathBuf,
    pub from_yaml_layers: bool,
}

pub struct ConfigPlugin {
    pub config_path: Option<PathBuf>,
}

impl Plugin for ConfigPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "config"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let loaded = load_app_config_detailed(self.config_path.as_deref())?;
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let agent_dir = dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(".xylitol");
        ctx.provide(ConfigService {
            app: loaded.config,
            agent_dir,
            cwd,
            from_yaml_layers: loaded.from_yaml_layers,
        })?;
        Ok(())
    }
}
