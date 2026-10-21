use std::sync::Arc;

use xylitol_official_plugin_config::plugin::ConfigService;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, TypedPlugin};

pub struct ObsPlugin;

impl TypedPlugin for ObsPlugin {
    type Deps = (Arc<ConfigService>,);

    fn name(&self) -> &str {
        "obs"
    }

    fn apply<'a>(
        &'a self,
        _: &'a Ctx,
        (cfg,): Self::Deps,
    ) -> BoxFuture<'a, Result<xylitol_plugin_sdk::rt::Effect, CordisError>> {
        Box::pin(async move {
            crate::init_logging(&cfg.agent_dir, &cfg.app.otel);
            Ok(Effect::Done)
        })
    }
}
