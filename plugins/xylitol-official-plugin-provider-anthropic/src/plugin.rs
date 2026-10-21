use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin, PluginFactory};

pub struct AnthropicDialect;

pub struct AnthropicFactory;

impl PluginFactory<EmptyPluginConfig> for AnthropicFactory {
    fn name(&self) -> &str {
        "provider-anthropic"
    }

    fn build(&self, _: &EmptyPluginConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(AnthropicPlugin))
    }
}

struct AnthropicPlugin;

impl Plugin for AnthropicPlugin {
    fn name(&self) -> &str {
        "provider-anthropic"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(AnthropicDialect)?;
            Ok(Effect::Done)
        })
    }
}
