use std::sync::Arc;

use xylitol_official_plugin_config::plugin::ConfigService;
use xylitol_plugin_sdk::TrustPreLoad;
use xylitol_plugin_sdk::rt::{
    BoxFuture, Call, CallExt, CordisError, Ctx, Effect, EventKey, Listener, TypedPlugin,
};
use xylitol_runtime::trust::TrustManager;

pub struct TrustPlugin;

impl TypedPlugin for TrustPlugin {
    type Deps = (Arc<ConfigService>,);

    fn name(&self) -> &str {
        "trust"
    }

    fn apply<'a>(
        &'a self,
        ctx: &'a Ctx,
        (cfg,): Self::Deps,
    ) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            let mgr = TrustManager::new(cfg.agent_dir.clone());
            ctx.provide(mgr.clone())?;
            ctx.events()
                .on(ctx, &EventKey::<TrustPreLoad>::of(), TrustGate { mgr })?;
            Ok(Effect::Done)
        })
    }
}

struct TrustGate {
    mgr: TrustManager,
}

impl Listener<TrustPreLoad> for TrustGate {
    fn call<'a>(
        &'a self,
        _: &'a Ctx,
        event: &'a TrustPreLoad,
        call: Call<'a, Option<String>>,
    ) -> BoxFuture<'a, Result<Option<Option<String>>, CordisError>> {
        Box::pin(async move {
            if self.mgr.is_trusted(&event.cwd) {
                call.chain().await
            } else {
                Ok(Some(Some("project is not trusted".into())))
            }
        })
    }
}
