use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct DriverMarker;

pub struct DriverPlugin;

impl Plugin for DriverPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "driver"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(DriverMarker)?;
        Ok(())
    }
}
