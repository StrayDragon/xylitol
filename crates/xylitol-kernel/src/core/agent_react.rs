use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub struct AgentReact;

pub struct AgentReactPlugin;

impl Plugin for AgentReactPlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "agent-react"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(AgentReact)?;
        Ok(())
    }
}
