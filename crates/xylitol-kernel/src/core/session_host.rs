use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin};

pub struct SessionHost;

pub struct SessionHostPlugin;

impl Plugin for SessionHostPlugin {
    fn name(&self) -> &str {
        "session-host"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(SessionHost)?;
            Ok(Effect::Done)
        })
    }
}
