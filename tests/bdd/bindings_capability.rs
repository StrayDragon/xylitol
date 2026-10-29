use crate::bdd::fixtures::*;
use crate::bdd::steps_capability::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "覆盖于下一回合生效"
)]
async fn test_c2998_overlay_next_turn(
    agent: AgentState,
    ws: Workspace,
    capability_bdd: CapabilityBdd,
) {
}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "覆盖不跨会话"
)]
fn test_c2998_overlay_session_scoped(
    agent: AgentState,
    ws: Workspace,
    capability_bdd: CapabilityBdd,
) {
}
