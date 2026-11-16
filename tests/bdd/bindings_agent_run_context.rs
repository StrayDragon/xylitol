use crate::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-run-context.feature",
    name = "上下文作为种子"
)]
async fn test_run_context_seed(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-run-context.feature",
    name = "上下文不落盘"
)]
async fn test_run_context_not_persisted(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-run-context.feature",
    name = "系统提示覆盖"
)]
async fn test_run_context_system_override(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-run-context.feature",
    name = "本轮仍落盘"
)]
async fn test_run_context_current_turn(agent: AgentState, ws: Workspace) {}
