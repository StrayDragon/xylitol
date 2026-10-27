use xylitol_plugin_sdk::LoopMiddlewareRegistry;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::config::LoopExampleConfig;
use crate::middleware::{
    DEFAULT_PLAN_SETTLES, example_middleware, plan_state_middleware, progress_reminder_middleware,
};

pub struct LoopExamplePlugin {
    config: LoopExampleConfig,
}

impl LoopExamplePlugin {
    pub fn new(config: &LoopExampleConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for LoopExamplePlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "loop-example"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        if !self.config.is_active() {
            return Ok(());
        }
        let Some(registry) = ctx.get::<LoopMiddlewareRegistry>() else {
            return Ok(());
        };
        let mut handles = vec![
            registry
                .register(example_middleware(&self.config))
                .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
        ];
        if self.config.plan_state {
            let max_settles = self.config.max_settles.unwrap_or(DEFAULT_PLAN_SETTLES);
            handles.push(
                registry
                    .register(plan_state_middleware(max_settles))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        if self.config.progress_reminder {
            handles.push(
                registry
                    .register(progress_reminder_middleware(&self.config))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        ctx.effect(|| async move {
            for handle in handles {
                handle.unregister();
            }
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

    fn enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            enabled: true,
            max_settles: Some(2),
            deny_bash_patterns: vec!["rm -rf /".into()],
            plan_state: false,
            progress_reminder: false,

            progress_reminder_stale_after: 3,
        }
    }

    fn plan_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            plan_state: true,
            progress_reminder: false,

            progress_reminder_stale_after: 3,
            ..enabled()
        }
    }

    fn reminder_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            progress_reminder: true,
            ..enabled()
        }
    }

    fn ctx_with_registry() -> (Ctx, Arc<LoopMiddlewareRegistry>) {
        let ctx = Ctx::root().expect("root");
        let registry = Arc::new(LoopMiddlewareRegistry::new());
        ctx.provide_as::<LoopMiddlewareRegistry>(registry.clone())
            .expect("provide registry");
        (ctx, registry)
    }

    #[tokio::test]
    async fn disabled_config_registers_nothing() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&LoopExampleConfig::default()).expect("new"));
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty());
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn enabled_config_registers_and_unload_leaves_no_residue() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&enabled()).expect("new"));
        for _ in 0..32 {
            if !registry.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(registry.ids(), vec!["loop-example".to_string()]);
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty());
        assert!(registry.snapshot().before_tool.is_empty());
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn plan_state_registers_both_and_unloads_both() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&plan_enabled()).expect("new"));
        for _ in 0..64 {
            if registry.ids().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            registry.ids(),
            vec!["loop-example".to_string(), "plan-state".to_string()],
            "plan mode must register both middlewares"
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty(), "unload must remove both middlewares");
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn progress_reminder_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&reminder_enabled()).expect("new"));
        for _ in 0..64 {
            if registry.ids().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            registry.ids(),
            vec!["loop-example".to_string(), "progress-reminder".to_string()],
            "progress reminder must register alongside the base middleware"
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty(), "unload must remove both middlewares");
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn missing_registry_is_a_noop() {
        let ctx = Ctx::root().expect("root");
        let fiber = ctx.plugin(LoopExamplePlugin::new(&enabled()).expect("new"));
        fiber.dispose().await.expect("dispose");
        let _ = ctx.shutdown().await;
    }
}
