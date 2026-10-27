use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::plugin_events::{
    SessionPrepare, SessionPrepareReason, SessionPrepareResult,
};
use xylitol_plugin_sdk::rt::{CallExt, Ctx, Plugin, Result};
use xylitol_plugin_sdk::session_prepare::SessionPrepareHub;

use crate::config::McpServerConfig;
use crate::hub::McpPrepareHub;

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(default)]
pub struct McpPluginConfig {
    pub servers: Vec<McpServerConfig>,
}

pub struct McpPlugin {
    servers: Vec<McpServerConfig>,
}

impl McpPlugin {
    pub fn new(config: &McpPluginConfig) -> Result<Self> {
        Ok(Self {
            servers: config.servers.clone(),
        })
    }
}

impl Plugin for McpPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "mcp"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        if !self.servers.is_empty()
            && let Some(registry) = ctx.get::<xylitol_plugin_sdk::LoopMiddlewareRegistry>()
        {
            let handle = registry
                .register(Arc::new(crate::truncate::McpResultTruncate::new()))
                .map_err(|e| xylitol_plugin_sdk::rt::ErrorKind::PluginFailed(Box::new(e)))?;
            ctx.effect(|| async move {
                handle.unregister();
                Ok(())
            })?;
        }
        let hub = McpPrepareHub::new(self.servers.clone());
        let hub_arc: Arc<dyn SessionPrepareHub> = Arc::new(hub.clone());
        ctx.provide_as::<dyn SessionPrepareHub>(hub_arc)?;
        ctx.on::<SessionPrepare>({
            let hub = hub.clone();
            move |event, call| {
                let hub = hub.clone();
                async move {
                    if event.reason != SessionPrepareReason::Generate {
                        hub.cue_connecting();
                        hub.reset_join().await;
                    }
                    let mine = hub.ensure().await;
                    if let Some(regs) = call.ctx().get::<HostRegistries>() {
                        for tool in hub.tools() {
                            let name = tool.name().to_string();
                            regs.tools.insert(name, tool);
                        }
                    }
                    let mut acc = call.chain().await?.unwrap_or_default();
                    merge_prepare(&mut acc, mine);
                    Ok(Some(acc))
                }
            }
        })?;
        Ok(())
    }
}

fn merge_prepare(acc: &mut SessionPrepareResult, mine: SessionPrepareResult) {
    if acc.fingerprint.is_empty() {
        *acc = mine;
        return;
    }
    if mine.fingerprint.is_empty() {
        return;
    }
    acc.fingerprint = format!("{}\n{}", acc.fingerprint, mine.fingerprint);
    if !mine.env_body.is_empty() {
        if acc.env_body.is_empty() {
            acc.env_body = mine.env_body;
        } else {
            acc.env_body = format!("{}\n{}", acc.env_body, mine.env_body);
        }
    }
}
