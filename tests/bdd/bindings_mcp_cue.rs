use crate::bdd::steps_mcp_cue::{McpCueBdd, mcp_cue_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "settled snapshot 清 cue"
)]
fn test_settled_snapshot_clears_cue(mcp_cue_bdd: McpCueBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "gate timeout 收口"
)]
fn test_gate_timeout_settles(mcp_cue_bdd: McpCueBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-mcp/infra-mcp.feature",
    name = "挂死 server 超时收口"
)]
fn test_hanging_server_settles(mcp_cue_bdd: McpCueBdd) {}
