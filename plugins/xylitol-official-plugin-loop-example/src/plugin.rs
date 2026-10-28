use xylitol_plugin_sdk::LoopMiddlewareRegistry;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::config::LoopExampleConfig;
use crate::middleware::{
    DEFAULT_TODO_PROGRESS_SETTLES, example_middleware, generator_critic_middleware,
    plan_mode_middleware, self_heal_middleware, todo_checkpoint_middleware,
    todo_progress_middleware, todo_reminder_middleware, tool_exposure_middleware,
    workflow_middleware,
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
        if self.config.plan_mode {
            let store = ctx
                .get::<xylitol_plugin_sdk::RuntimeSessionStore>()
                .map(|store| store.0.clone())
                .or_else(|| ctx.get::<dyn XySessionStore>());
            handles.push(
                registry
                    .register(plan_mode_middleware(store, Some(ctx.clone())))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        if self.config.todo_progress {
            let max_settles = self
                .config
                .max_settles
                .unwrap_or(DEFAULT_TODO_PROGRESS_SETTLES);
            handles.push(
                registry
                    .register(todo_progress_middleware(max_settles))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        if self.config.todo_checkpoint {
            let store = ctx
                .get::<xylitol_plugin_sdk::RuntimeSessionStore>()
                .map(|store| store.0.clone())
                .or_else(|| ctx.get::<dyn XySessionStore>());
            handles.push(
                registry
                    .register(todo_checkpoint_middleware(store, Some(ctx.clone())))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        if self.config.todo_reminder {
            handles.push(
                registry
                    .register(todo_reminder_middleware(&self.config))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        if self.config.workflow.enabled {
            let store = ctx
                .get::<xylitol_plugin_sdk::RuntimeSessionStore>()
                .map(|store| store.0.clone())
                .or_else(|| ctx.get::<dyn XySessionStore>());
            handles.push(
                registry
                    .register(workflow_middleware(
                        store,
                        Some(ctx.clone()),
                        &self.config.workflow,
                    ))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
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
            plan_mode: false,
            max_settles: Some(2),
            deny_bash_patterns: vec!["rm -rf /".into()],
            todo_progress: false,
            todo_reminder: false,
            todo_reminder_stale_after: 3,
            todo_checkpoint: false,
            self_heal: Default::default(),
            generator_critic: Default::default(),
            workflow: Default::default(),
            tool_exposure: Default::default(),
        }
    }

    fn plan_mode_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            plan_mode: true,
            ..enabled()
        }
    }

    fn plan_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            todo_progress: true,
            todo_reminder: false,
            todo_reminder_stale_after: 3,
            todo_checkpoint: false,
            self_heal: Default::default(),
            generator_critic: Default::default(),
            workflow: Default::default(),
            tool_exposure: Default::default(),
            ..enabled()
        }
    }

    fn reminder_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            todo_reminder: true,
            ..enabled()
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

    fn checkpoint_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            todo_checkpoint: true,
            self_heal: Default::default(),
            generator_critic: Default::default(),
            workflow: Default::default(),
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

    fn workflow_enabled() -> LoopExampleConfig {
        LoopExampleConfig {
            workflow: crate::config::WorkflowConfig {
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
    async fn plan_mode_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&plan_mode_enabled()).expect("new"));
        for _ in 0..64 {
            if registry.ids().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            registry.ids(),
            vec!["loop-example".to_string(), "plan-mode".to_string()],
            "plan mode must register alongside the base middleware"
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty(), "unload must remove both middlewares");
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn todo_progress_registers_both_and_unloads_both() {
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
            vec!["loop-example".to_string(), "todo-progress".to_string()],
            "plan mode must register both middlewares"
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty(), "unload must remove both middlewares");
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn todo_reminder_registers_and_unloads() {
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
            vec!["loop-example".to_string(), "todo-reminder".to_string()],
            "progress reminder must register alongside the base middleware"
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty(), "unload must remove both middlewares");
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
    async fn todo_checkpoint_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&checkpoint_enabled()).expect("new"));
        for _ in 0..64 {
            if registry.ids().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            registry.ids(),
            vec!["loop-example".to_string(), "todo-checkpoint".to_string()],
            "plan checkpoint must register alongside the base middleware"
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
    async fn workflow_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(LoopExamplePlugin::new(&workflow_enabled()).expect("new"));
        for _ in 0..64 {
            if registry.ids().len() == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            registry.ids(),
            vec!["loop-example".to_string(), "workflow".to_string()],
            "workflow must register alongside the base middleware"
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
