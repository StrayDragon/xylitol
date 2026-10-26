use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct OpenAiDialect;

pub struct OpenAiPlugin;

impl OpenAiPlugin {
    pub fn new(_: &EmptyPluginConfig) -> Result<Self> {
        Ok(Self)
    }
}

impl Plugin for OpenAiPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "provider-openai"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(OpenAiDialect)?;
        Ok(())
    }
}
