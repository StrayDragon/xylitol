use std::sync::Arc;

use xylitol_official_plugin_config::plugin::ConfigService;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct ObsPlugin;

impl Plugin for ObsPlugin {
    type Deps = Arc<ConfigService>;

    fn name(&self) -> &str {
        "obs"
    }

    async fn apply(&self, _: &Ctx, cfg: Arc<ConfigService>) -> Result {
        let otel: xylitol_official_plugin_config::types::OtelConfig =
            cfg.app.section_or_default("otel").unwrap_or_default();
        crate::init_logging(&cfg.agent_dir, &otel);
        Ok(())
    }
}
