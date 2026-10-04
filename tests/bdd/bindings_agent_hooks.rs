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

// ── c2835 后继：agent-hooks 裸规则回填 ──────────────────────────────

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "convention-event-names-dispatch"
)]
async fn test_hk1_convention_events(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "qualifier-narrows-dispatch"
)]
async fn test_hk2_qualifier_match(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "stdin-receives-event-json"
)]
async fn test_hk3_stdin_event_json(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "provider-headers-before-send"
)]
async fn test_hk4_provider_headers(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "provider-hooks-empty-dispatch-noop"
)]
async fn test_hk5_provider_hooks_noop(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "react-script-bridge-tool-call"
)]
async fn test_hk6_react_script_bridge(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "lifecycle-hooks-settle-round"
)]
async fn test_hk7_lifecycle_hooks(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "model-and-thinking-dispatch"
)]
async fn test_hk8_model_thinking_dispatch(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "tree-and-switch-before-block"
)]
async fn test_hk9_tree_switch_before_block(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "user-bash-blocks-run"
)]
async fn test_hk10_user_bash_block(agent: AgentState) {}
