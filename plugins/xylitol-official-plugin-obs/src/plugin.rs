use std::sync::Arc;

use xylitol_official_plugin_config::plugin::ConfigService;
use xylitol_plugin_sdk::config_section::SectionBinding;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct ObsPlugin;

impl Plugin for ObsPlugin {
    type Deps = Arc<ConfigService>;

    fn name(&self) -> &str {
        "obs"
    }

    async fn apply(&self, _: &Ctx, cfg: Arc<ConfigService>) -> Result {
        let otel = crate::config::OtelConfig::try_from_sections(&cfg.app.sections)
            .unwrap_or_default()
            .unwrap_or_default();
        crate::init_logging(&cfg.agent_dir, &otel);
        Ok(())
    }
}
