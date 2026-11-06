use std::sync::Arc;

use xylitol_plugin_sdk::CodemodeConfig;
use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::LoopMiddlewareRegistry;
use xylitol_plugin_sdk::rt::{Ctx, ErrorKind, Plugin, Result};

use crate::declarations::{descriptors_from_tools, render_declarations};
use crate::middleware::CodemodeMiddleware;
use crate::runner::CodemodeRunner;
use crate::tool::RunCodeTool;

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::rt::CtxHost;

    fn ctx_with_registries() -> Ctx {
        let ctx = Ctx::root().expect("root");
        ctx.provide(HostRegistries::new()).expect("registries");
        ctx
    }

    #[tokio::test]
    async fn disabled_registers_nothing() {
        let ctx = ctx_with_registries();
        let fiber = ctx.plugin(CodemodePlugin::new(&CodemodeConfig::default()).expect("plugin"));
        fiber.dispose().await.expect("dispose");
        let registries = ctx.get::<HostRegistries>().expect("registries");
        assert!(registries.tools.get("run_code").is_none());
        let _ = ctx.shutdown().await;
    }

    #[tokio::test]
    async fn enabled_registers_run_code() {
        let ctx = ctx_with_registries();
        let config = CodemodeConfig {
            enabled: true,
            ..Default::default()
        };
        let fiber = ctx.plugin(CodemodePlugin::new(&config).expect("plugin"));
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
        let registries = ctx.get::<HostRegistries>().expect("registries");
        assert!(registries.tools.get("run_code").is_some());
        fiber.dispose().await.expect("dispose");
        let _ = ctx.shutdown().await;
    }
}

pub struct CodemodePlugin {
    config: CodemodeConfig,
}

impl CodemodePlugin {
    pub fn new(config: &CodemodeConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

impl Plugin for CodemodePlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "codemode"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        if !self.config.is_active() {
            return Ok(());
        }
        xylitol_plugin_sdk::set_codemode_max_nested_calls(self.config.max_nested_calls);
        xylitol_plugin_sdk::set_codemode_exposure_policy(self.config.tool_exposure.clone());
        xylitol_plugin_sdk::set_codemode_exposure_overrides(self.config.exposure.clone());
        let runner = Arc::new(CodemodeRunner::new(self.config.clone()).map_err(|err| {
            ErrorKind::PluginFailed(Box::new(std::io::Error::other(err.to_string())))
        })?);
        let Some(registries) = ctx.get::<HostRegistries>() else {
            return Ok(());
        };
        registries
            .tools
            .insert("run_code", Arc::new(RunCodeTool::new(runner)));
        let descriptors = descriptors_from_tools(&registries.tools.snapshot());
        let declarations = render_declarations(&descriptors, self.config.inline_budget);
        if !declarations.is_empty()
            && let Some(registry) = ctx.get::<LoopMiddlewareRegistry>()
        {
            let handle = registry
                .register(Arc::new(CodemodeMiddleware::new(declarations)))
                .map_err(|err| {
                    ErrorKind::PluginFailed(Box::new(std::io::Error::other(err.to_string())))
                })?;
            ctx.effect(|| async move {
                handle.unregister();
                Ok(())
            })?;
        }
        Ok(())
    }
}
