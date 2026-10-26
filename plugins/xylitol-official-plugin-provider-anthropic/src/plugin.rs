use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct AnthropicDialect;

pub struct AnthropicPlugin;

impl AnthropicPlugin {
    pub fn new(_: &EmptyPluginConfig) -> Result<Self> {
        Ok(Self)
    }
}

impl Plugin for AnthropicPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "provider-anthropic"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(AnthropicDialect)?;
        Ok(())
    }
}
