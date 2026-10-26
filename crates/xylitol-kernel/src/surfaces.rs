use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct PrintPlugin;

impl PrintPlugin {
    pub fn new(_: &EmptyPluginConfig) -> Result<Self> {
        Ok(Self)
    }
}

impl Plugin for PrintPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "app-print"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(PrintSurface)?;
        Ok(())
    }
}

pub struct PrintSurface;

pub struct ServePlugin;

impl ServePlugin {
    pub fn new(_: &EmptyPluginConfig) -> Result<Self> {
        Ok(Self)
    }
}

impl Plugin for ServePlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "serve"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(ServeSurface)?;
        Ok(())
    }
}

pub struct ServeSurface;
