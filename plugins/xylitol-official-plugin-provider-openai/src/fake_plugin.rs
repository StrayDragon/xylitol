//! `PluginFactory` entry: fake provider (tests / profile only).

use xylitol_plugin_sdk::EmptyPluginConfig;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin, PluginFactory};

/// Marker that the fake dialect plugin is mounted.
pub struct FakeDialect;

pub struct FakeFactory;

impl PluginFactory<EmptyPluginConfig> for FakeFactory {
    fn name(&self) -> &str {
        "provider-fake"
    }

    fn build(&self, _: &EmptyPluginConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(FakePlugin))
    }
}

struct FakePlugin;

impl Plugin for FakePlugin {
    fn name(&self) -> &str {
        "provider-fake"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(FakeDialect)?;
            Ok(Effect::Done)
        })
    }
}
