use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use xylitol_plugin_sdk::HostRegistries;
use xylitol_plugin_sdk::plugin_events::{
    SessionPrepare, SessionPrepareReason, SessionPrepareResult,
};
use xylitol_plugin_sdk::rt::{
    BoxFuture, Call, CordisError, Ctx, Effect, EventKey, Listener, Plugin, PluginFactory,
};
use xylitol_plugin_sdk::session_prepare::SessionPrepareHub;
use xylitol_plugin_sdk::session_prepare_hub_key;

use crate::config::McpServerConfig;
use crate::hub::McpPrepareHub;

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
            let hub = McpPrepareHub::new(self.servers.clone());
            let hub_arc: Arc<dyn SessionPrepareHub> = Arc::new(hub.clone());
            ctx.provide_as(session_prepare_hub_key(), hub_arc)?;
            ctx.events().on(
                ctx,
                &EventKey::<SessionPrepare>::of(),
                McpPrepareListener { hub },
            )?;
            Ok(Effect::Done)
        })
    }
}

struct McpPrepareListener {
    hub: McpPrepareHub,
}

impl Listener<SessionPrepare> for McpPrepareListener {
    fn call<'a>(
        &'a self,
        ctx: &'a Ctx,
        event: &'a SessionPrepare,
        call: Call<'a, SessionPrepareResult>,
    ) -> BoxFuture<'a, Result<Option<SessionPrepareResult>, CordisError>> {
        Box::pin(async move {
            if event.reason != SessionPrepareReason::Generate {
                self.hub.cue_connecting();
                self.hub.reset_join().await;
            }
            let mine = self.hub.ensure().await;
            if let Some(regs) = ctx.get::<HostRegistries>() {
                for tool in self.hub.tools() {
                    let name = tool.name().to_string();
                    regs.tools.insert(name, tool);
                }
            }
            let mut acc = call.next().await?.unwrap_or_default();
            merge_prepare(&mut acc, mine);
            Ok(Some(acc))
        })
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
