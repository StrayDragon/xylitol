//! `PluginFactory` entry: OpenAI-compatible dialect is present in the process.

use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin, PluginFactory};

/// Marker that the OpenAI dialect plugin is mounted.
pub struct OpenAiDialect;

pub struct OpenAiFactory;

impl PluginFactory<EmptyPluginConfig> for OpenAiFactory {
    fn name(&self) -> &str {
        "provider-openai"
    }

    fn build(&self, _: &EmptyPluginConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(OpenAiPlugin))
    }
}

struct OpenAiPlugin;

impl Plugin for OpenAiPlugin {
    fn name(&self) -> &str {
        "provider-openai"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(OpenAiDialect)?;
            Ok(Effect::Done)
        })
    }
}
