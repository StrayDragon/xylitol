use crate::tests::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "pre-tool-call"
)]
async fn test_hook_pre(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "hook-blocks"
)]
async fn test_hook_block(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "hook-modifies-args"
)]
async fn test_hook_modify_args(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "three-layer-merge"
)]
async fn test_hook_merge(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "hook-timeout"
)]
async fn test_hook_timeout(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "before-provider-request"
)]
async fn test_hook_provider_request(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "after-provider-response"
)]
async fn test_hook_provider_response(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "empty-hooks-noop"
)]
async fn test_hook_empty_noop(agent: AgentState) {}
