use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin, PluginFactory};

pub struct PrintFactory;

impl PluginFactory<EmptyPluginConfig> for PrintFactory {
    fn name(&self) -> &str {
        "app-print"
    }

    fn build(&self, _: &EmptyPluginConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(PrintPlugin))
    }
}

struct PrintPlugin;

impl Plugin for PrintPlugin {
    fn name(&self) -> &str {
        "app-print"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(PrintSurface)?;
            Ok(Effect::Done)
        })
    }
}

pub struct PrintSurface;

pub struct ServeFactory;

impl PluginFactory<EmptyPluginConfig> for ServeFactory {
    fn name(&self) -> &str {
        "serve"
    }

    fn build(&self, _: &EmptyPluginConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(ServePlugin))
    }
}

struct ServePlugin;

impl Plugin for ServePlugin {
    fn name(&self) -> &str {
        "serve"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(ServeSurface)?;
            Ok(Effect::Done)
        })
    }
}

pub struct ServeSurface;
