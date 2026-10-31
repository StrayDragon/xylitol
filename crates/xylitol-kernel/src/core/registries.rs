use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct RegistriesPlugin;

fn provide_reload_hub(ctx: &Ctx) -> Result {
    ctx.provide(xylitol_plugin_sdk::ReloadHub::new())
        .map(|_| ())
}

impl Plugin for RegistriesPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "registries"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(HostRegistries::new())?;
        provide_reload_hub(ctx)?;
        Ok(())
    }
}
