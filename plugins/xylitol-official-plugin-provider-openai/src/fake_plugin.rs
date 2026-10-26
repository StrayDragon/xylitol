use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct FakeDialect;

pub struct FakePlugin;

impl FakePlugin {
    pub fn new(_: &EmptyPluginConfig) -> Result<Self> {
        Ok(Self)
    }
}

impl Plugin for FakePlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "provider-fake"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(FakeDialect)?;
        Ok(())
    }
}
