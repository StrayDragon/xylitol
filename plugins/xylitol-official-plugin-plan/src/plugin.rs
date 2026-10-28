use xylitol_plugin_sdk::LoopMiddlewareRegistry;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::config::PlanConfig;
use crate::middleware::plan_mode_middleware;

pub struct PlanPlugin {
    config: PlanConfig,
}

impl PlanPlugin {
    pub fn new(config: &PlanConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for PlanPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "plan"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        if !self.config.is_active() {
            return Ok(());
        }
        let Some(registry) = ctx.get::<LoopMiddlewareRegistry>() else {
            return Ok(());
        };
        let store = ctx
            .get::<xylitol_plugin_sdk::RuntimeSessionStore>()
            .map(|store| store.0.clone())
            .or_else(|| ctx.get::<dyn XySessionStore>());
        let handle = registry
            .register(plan_mode_middleware(store, Some(ctx.clone())))
            .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?;
        ctx.effect(|| async move {
            handle.unregister();
            Ok(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use xylitol_plugin_sdk::rt::CtxHost;

    use super::*;

    fn ctx_with_registry() -> (Ctx, Arc<LoopMiddlewareRegistry>) {
        let ctx = Ctx::root().expect("root");
        let registry = Arc::new(LoopMiddlewareRegistry::new());
        ctx.provide_as::<LoopMiddlewareRegistry>(registry.clone())
            .expect("provide registry");
        (ctx, registry)
    }

    #[tokio::test]
    async fn disabled_registers_nothing() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(
            PlanPlugin::new(&PlanConfig {
                enabled: false,
                ..Default::default()
            })
            .expect("new"),
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty());
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn enabled_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(PlanPlugin::new(&PlanConfig::default()).expect("new"));
        for _ in 0..64 {
            if !registry.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(registry.ids(), vec!["plan-mode".to_string()]);
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty());
        let _ = ctx.shutdown().await;
    }
}
