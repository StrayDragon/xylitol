use xylitol_plugin_sdk::rt::{BoxFuture, CordisError, Ctx, Effect, Plugin};

pub use xylitol_runtime::agent::MCP_FIRST_TURN_GATE_TIMEOUT as FIRST_TURN_GATE_TIMEOUT;

pub struct ToolTable;

pub struct ToolTablePlugin;

impl Plugin for ToolTablePlugin {
    fn name(&self) -> &str {
        "tool-table"
    }

    fn apply<'a>(&'a self, ctx: &'a Ctx) -> BoxFuture<'a, Result<Effect, CordisError>> {
        Box::pin(async move {
            ctx.provide(ToolTable)?;
            Ok(Effect::Done)
        })
    }
}
