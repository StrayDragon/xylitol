use std::sync::Arc;

use xylitol_official_plugin_config::plugin::ConfigService;
use xylitol_plugin_sdk::TrustPreLoad;
use xylitol_plugin_sdk::rt::{CallExt, Ctx, Plugin, Result};
use xylitol_runtime::trust::TrustManager;

pub struct TrustPlugin;

impl Plugin for TrustPlugin {
    type Deps = Arc<ConfigService>;

    fn name(&self) -> &str {
        "trust"
    }

    async fn apply(&self, ctx: &Ctx, cfg: Arc<ConfigService>) -> Result {
        let mgr = TrustManager::new(cfg.agent_dir.clone());
        ctx.provide(mgr.clone())?;
        ctx.on::<TrustPreLoad>({
            let mgr = mgr.clone();
            move |event, call| {
                let mgr = mgr.clone();
                async move {
                    if mgr.is_trusted(&event.cwd) {
                        call.chain().await
                    } else {
                        Ok(Some(Some("project is not trusted".into())))
                    }
                }
            }
        })?;
        Ok(())
    }
}
