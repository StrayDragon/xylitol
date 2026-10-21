//! `PluginFactory` entry: MCP client (config-gated, zero servers = no-op).

use std::sync::Arc;
use std::time::Duration;

use schemars::JsonSchema;
use serde::Deserialize;
use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin, PluginFactory};

use crate::assemble::connect_and_discover;
use crate::config::McpServerConfig;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(default)]
pub struct McpPluginConfig {
    pub servers: Vec<McpServerConfig>,
}

pub struct McpFactory;

impl PluginFactory<McpPluginConfig> for McpFactory {
    fn name(&self) -> &str {
        "mcp"
    }

    fn build(&self, config: &McpPluginConfig) -> Result<Box<dyn Plugin>, CordisError> {
        Ok(Box::new(McpPlugin {
            servers: config.servers.clone(),
        }))
    }
}

struct McpPlugin {
    servers: Vec<McpServerConfig>,
}

impl Plugin for McpPlugin {
    fn name(&self) -> &str {
        "mcp"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            if self.servers.is_empty() {
                return Ok(Effect::Done);
            }
            let discovered =
                tokio::time::timeout(CONNECT_TIMEOUT, connect_and_discover(&self.servers))
                    .await
                    .ok()
                    .flatten();
            let Some((_manager, tools)) = discovered else {
                return Ok(Effect::Done);
            };
            let Some(regs) = ctx.get::<HostRegistries>() else {
                return Ok(Effect::Done);
            };
            let mut names = Vec::new();
            for tool in tools {
                let name = tool.name().to_string();
                regs.tools.insert(name.clone(), tool);
                names.push(name);
            }
            let _ = Arc::clone(&regs);
            Ok(Effect::Disposer(Box::new(move || {
                for name in names {
                    regs.tools.remove(&name);
                }
                Ok(())
            })))
        })
    }
}
