use crate::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "预工具调用"
)]
async fn test_hook_pre(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "钩子块"
)]
async fn test_hook_block(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "钩子修改参数"
)]
async fn test_hook_modify_args(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "三个层合并"
)]
async fn test_hook_merge(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "钩子超时"
)]
async fn test_hook_timeout(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "之前厂商请求"
)]
async fn test_hook_provider_request(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "之后厂商应答"
)]
async fn test_hook_provider_response(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "空钩子空操作"
)]
async fn test_hook_empty_noop(agent: AgentState) {}

// ── c2835 后继：agent-hooks 裸规则回填 ──────────────────────────────

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "约定事件名称分发"
)]
async fn test_hk1_convention_events(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "限定符收窄分发"
)]
async fn test_hk2_qualifier_match(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "stdin接收事件JSON"
)]
async fn test_hk3_stdin_event_json(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "厂商头之前发送"
)]
async fn test_hk4_provider_headers(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "厂商钩子空分发空操作"
)]
async fn test_hk5_provider_hooks_noop(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "ReAct脚本桥缝工具调用"
)]
async fn test_hk6_react_script_bridge(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "生命周期钩子结算回合"
)]
async fn test_hk7_lifecycle_hooks(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "模型与思考分发"
)]
async fn test_hk8_model_thinking_dispatch(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "树与切换之前块"
)]
async fn test_hk9_tree_switch_before_block(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/agent-hooks/agent-hooks.feature",
    name = "用户bash块运行"
)]
async fn test_hk10_user_bash_block(agent: AgentState) {}
