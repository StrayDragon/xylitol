use std::sync::Arc;

use xylitol_plugin_sdk::LoopMiddlewareRegistry;
use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct AgentReact;

pub struct AgentReactPlugin;

impl Plugin for AgentReactPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "agent-react"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        provide_react_context(ctx)
    }
}

fn provide_react_context(ctx: &Ctx) -> Result {
    ctx.provide(AgentReact)?;
    ctx.provide_as::<LoopMiddlewareRegistry>(Arc::new(LoopMiddlewareRegistry::new()))?;
    Ok(())
}
