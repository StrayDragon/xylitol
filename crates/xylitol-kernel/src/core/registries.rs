//! Core plugin: named contribution registries.

use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin};

pub struct RegistriesPlugin;

impl Plugin for RegistriesPlugin {
    fn name(&self) -> &str {
        "registries"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(HostRegistries::new())?;
            Ok(Effect::Done)
        })
    }
}
