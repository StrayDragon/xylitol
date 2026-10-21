//! Core plugin: per-session tool-table freeze + first-turn MCP gate.
//!
//! Freeze phase still lives on the bound runtime session (single-writer
//! invariant). This plugin owns the first-turn timeout budget; the driver
//! plugin's MCP poll implements the gate with that budget.

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
