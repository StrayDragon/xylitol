use std::sync::Arc;

use xylitol_plugin_sdk::LoopMiddlewareRegistry;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

use crate::config::{ContextInjectConfig, DedupMode, InjectorPoint, Placement};
use crate::coordinator::ContextInjectMiddleware;
use crate::drift::DriftInjector;
use crate::registry::{ContextInjector, ContextInjectorRegistry, InjectorSource};

pub struct ContextInjectPlugin {
    config: ContextInjectConfig,
}

impl ContextInjectPlugin {
    pub fn new(config: &ContextInjectConfig) -> Result<Self> {
        config.validate().map_err(|message| {
            xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(std::io::Error::other(
                message,
            )))
        })?;
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for ContextInjectPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "context-inject"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let registry = Arc::new(ContextInjectorRegistry::new());
        registry
            .register(ContextInjector {
                id: "drift".into(),
                point: InjectorPoint::TurnPre,
                priority: 0,
                placement: Placement::HistoryRow,
                dedup: DedupMode::SuppressIfUnchanged,
                group: Some("system-reminder".into()),
                source: InjectorSource::Native(Arc::new(DriftInjector::default())),
            })
            .map_err(|error| {
                xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(std::io::Error::other(
                    error,
                )))
            })?;
        for injector in &self.config.merged() {
            registry
                .register(ContextInjector {
                    id: injector.id.clone(),
                    point: injector.point,
                    priority: 0,
                    placement: injector.placement,
                    dedup: injector.dedup,
                    group: injector.group.clone(),
                    source: InjectorSource::Command {
                        command: injector.command.clone(),
                        timeout_secs: injector.timeout_secs.unwrap_or(2),
                    },
                })
                .map_err(|error| {
                    xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(
                        std::io::Error::other(error),
                    ))
                })?;
        }
        ctx.provide_as::<ContextInjectorRegistry>(registry.clone())?;
        let Some(loop_registry) = ctx.get::<LoopMiddlewareRegistry>() else {
            return Ok(());
        };
        let store = ctx.get::<dyn XySessionStore>();
        let middleware = Arc::new(ContextInjectMiddleware::new(registry, store));
        let handle = loop_registry.register(middleware).map_err(|error| {
            xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(std::io::Error::other(
                error.to_string(),
            )))
        })?;
        ctx.effect(|| async move {
            handle.unregister();
            Ok(())
        })?;
        Ok(())
    }
}
