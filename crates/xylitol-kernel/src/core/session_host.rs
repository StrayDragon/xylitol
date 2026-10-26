use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct SessionHost;

pub struct SessionHostPlugin;

impl Plugin for SessionHostPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "session-host"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(SessionHost)?;
        Ok(())
    }
}
