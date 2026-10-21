use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin};

pub struct DriverMarker;

pub struct DriverPlugin;

impl Plugin for DriverPlugin {
    fn name(&self) -> &str {
        "driver"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(DriverMarker)?;
            Ok(Effect::Done)
        })
    }
}
