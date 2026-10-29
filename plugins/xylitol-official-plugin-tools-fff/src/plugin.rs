use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};
use xylitol_plugin_sdk::{EmptyPluginConfig, HostRegistries};

use crate::tools::fff_tools;

pub type FffPluginConfig = EmptyPluginConfig;

pub struct FffToolsPlugin;

impl FffToolsPlugin {
    pub fn new(_: &FffPluginConfig) -> Result<Self> {
        Ok(Self)
    }
}

impl Plugin for FffToolsPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "tools-fff"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let Some(regs) = ctx.get::<HostRegistries>() else {
            return Ok(());
        };
        let mut names = Vec::new();
        for tool in fff_tools() {
            let name = tool.name().to_string();
            regs.tools.insert(name.clone(), tool);
            names.push(name);
        }
        ctx.effect(|| async move {
            for name in names {
                regs.tools.remove(&name);
            }
            Ok(())
        })?;
        Ok(())
    }
}
