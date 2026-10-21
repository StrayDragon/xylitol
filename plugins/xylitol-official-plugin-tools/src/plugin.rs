//! `PluginFactory` entry: built-in tools.

use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin, PluginFactory};
use xylitol_plugin_sdk::{EmptyPluginConfig, HostRegistries};

use crate::default_tools;

pub type ToolsPluginConfig = EmptyPluginConfig;

pub struct ToolsFactory;

impl PluginFactory<ToolsPluginConfig> for ToolsFactory {
    fn name(&self) -> &str {
        "tools-builtin"
    }

    fn build(&self, _: &ToolsPluginConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(ToolsPlugin))
    }
}

struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn name(&self) -> &str {
        "tools-builtin"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            let Some(regs) = ctx.get::<HostRegistries>() else {
                return Ok(Effect::Done);
            };
            let mut names = Vec::new();
            for tool in default_tools() {
                let name = tool.name().to_string();
                regs.tools.insert(name.clone(), tool);
                names.push(name);
            }
            Ok(Effect::Disposer(Box::new(move || {
                for name in names {
                    regs.tools.remove(&name);
                }
                Ok(())
            })))
        })
    }
}
