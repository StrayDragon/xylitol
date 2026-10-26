use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};
use xylitol_plugin_sdk::{EmptyPluginConfig, HostRegistries};

use crate::default_tools;

pub type ToolsPluginConfig = EmptyPluginConfig;

pub struct ToolsPlugin;

impl ToolsPlugin {
    pub fn new(_: &ToolsPluginConfig) -> Result<Self> {
        Ok(Self)
    }
}

impl Plugin for ToolsPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "tools-builtin"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        let Some(regs) = ctx.get::<HostRegistries>() else {
            return Ok(());
        };
        let mut names = Vec::new();
        for tool in default_tools() {
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
