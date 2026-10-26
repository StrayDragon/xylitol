use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct RegistriesPlugin;

impl Plugin for RegistriesPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "registries"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(HostRegistries::new())?;
        Ok(())
    }
}
