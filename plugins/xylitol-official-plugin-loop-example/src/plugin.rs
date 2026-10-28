use xylitol_plugin_sdk::LoopMiddlewareRegistry;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::config::LoopExampleConfig;
use crate::middleware::{
    example_middleware, generator_critic_middleware, self_heal_middleware, tool_exposure_middleware,
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
        if self.config.generator_critic.enabled {
            handles.push(
                registry
                    .register(generator_critic_middleware(&self.config.generator_critic))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        if self.config.self_heal.enabled {
            handles.push(
                registry
                    .register(self_heal_middleware(&self.config.self_heal))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        if self.config.tool_exposure.enabled {
            handles.push(
                registry
                    .register(tool_exposure_middleware(&self.config.tool_exposure))
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
    use crate::config::ToolExposureConfig;

    fn enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            enabled: true,
            max_settles: Some(2),
            deny_bash_patterns: vec!["rm -rf /".into()],
            self_heal: Default::default(),
            generator_critic: Default::default(),
            tool_exposure: Default::default(),
        }
    }

    fn exposure_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            tool_exposure: ToolExposureConfig {
                enabled: true,
                allow: vec!["read".into()],
                ..Default::default()
            },
            ..enabled()
        }
    }

    fn self_heal_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            self_heal: crate::config::SelfHealConfig {
                enabled: true,
                ..Default::default()
            },
            ..enabled()
        }
    }

    fn critic_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            generator_critic: crate::config::GeneratorCriticConfig {
                enabled: true,
                ..Default::default()
            },
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
    async fn tool_exposure_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&exposure_enabled()).expect("new"));
        for _ in 0..64 {
            if registry.ids().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            registry.ids(),
            vec!["tool-exposure".to_string(), "loop-example".to_string()],
            "tool exposure must register alongside the base middleware"
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty(), "unload must remove both middlewares");
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn self_heal_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&self_heal_enabled()).expect("new"));
        for _ in 0..64 {
            if registry.ids().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            registry.ids(),
            vec!["loop-example".to_string(), "self-heal".to_string()],
            "self-heal must register alongside the base middleware"
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty(), "unload must remove both middlewares");
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn generator_critic_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&critic_enabled()).expect("new"));
        for _ in 0..64 {
            if registry.ids().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            registry.ids(),
            vec!["loop-example".to_string(), "generator-critic".to_string()],
            "generator-critic must register alongside the base middleware"
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
