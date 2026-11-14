use std::sync::Arc;

use xylitol_official_plugin_config::plugin::ConfigService;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};
use xylitol_runtime::trust::TrustManager;

pub struct TrustPlugin;

impl Plugin for TrustPlugin {
    type Deps = Arc<ConfigService>;

    fn name(&self) -> &str {
        "trust"
    }

    async fn apply(&self, ctx: &Ctx, cfg: Arc<ConfigService>) -> Result {
        let mgr = TrustManager::new(cfg.agent_dir.clone());
        ctx.provide(mgr)?;
        Ok(())
    }
}
