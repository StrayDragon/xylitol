use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin};

pub struct AgentReact;

pub struct AgentReactPlugin;

impl Plugin for AgentReactPlugin {
    fn name(&self) -> &str {
        "agent-react"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(AgentReact)?;
            Ok(Effect::Done)
        })
    }
}
