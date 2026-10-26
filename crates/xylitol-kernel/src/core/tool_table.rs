use xylitol_plugin_sdk::rt::{Ctx, Plugin, Result};

pub use xylitol_runtime::agent::MCP_FIRST_TURN_GATE_TIMEOUT as FIRST_TURN_GATE_TIMEOUT;

pub struct ToolTable;

pub struct ToolTablePlugin;

impl Plugin for ToolTablePlugin {
    type Deps = ();

    fn name(&self) -> &str {
        "tool-table"
    }

    async fn apply(&self, ctx: &Ctx, _: ()) -> Result {
        ctx.provide(ToolTable)?;
        Ok(())
    }
}
