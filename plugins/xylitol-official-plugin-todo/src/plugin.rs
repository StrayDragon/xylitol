use xylitol_plugin_sdk::LoopMiddlewareRegistry;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::config::TodoConfig;
use crate::middleware::{
    DEFAULT_TODO_PROGRESS_SETTLES, todo_checkpoint_middleware, todo_progress_middleware,
    todo_reminder_middleware,
};

pub struct TodoPlugin {
    config: TodoConfig,
}

impl TodoPlugin {
    pub fn new(config: &TodoConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for TodoPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "todo"
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
        let mut handles = Vec::new();
        if self.config.progress {
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
        if self.config.checkpoint {
            handles.push(
                registry
                    .register(todo_checkpoint_middleware(store, Some(ctx.clone())))
                    .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?,
            );
        }
        if self.config.reminder {
            handles.push(
                registry
                    .register(todo_reminder_middleware(&self.config))
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
            TodoPlugin::new(&TodoConfig {
                enabled: false,
                progress: true,
                ..Default::default()
            })
            .expect("new"),
        );
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty());
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn progress_registers_and_unloads() {
        let (ctx, registry) = ctx_with_registry();
        let fiber = ctx.plugin(
            TodoPlugin::new(&TodoConfig {
                enabled: true,
                progress: true,
                ..Default::default()
            })
            .expect("new"),
        );
        for _ in 0..64 {
            if !registry.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(registry.ids(), vec!["todo-progress".to_string()]);
        fiber.dispose().await.expect("dispose");
        assert!(registry.is_empty());
        let _ = ctx.shutdown().await;
    }
}
