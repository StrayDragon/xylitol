use crate::bdd::fixtures::*;
use rstest_bdd_macros::scenario;
use serial_test::serial;

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "中间件有序且卸载后消失"
)]
fn test_lm_order_and_unregister() {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "空链走基线"
)]
async fn test_lm_empty_chain_baseline(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "自定义停止结束运行"
)]
async fn test_lm_decide_stop(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "未支持交接报错"
)]
async fn test_lm_decide_handoff(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "中间件拒绝更严"
)]
async fn test_lm_before_tool_deny(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "改参不绕过权限"
)]
async fn test_lm_before_tool_rewrite_permission(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "无进展继续触发上限"
)]
async fn test_lm_continue_limit(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "观测可指认改动来源"
)]
#[serial]
async fn test_lm_trace_attribution(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "未配置零装配"
)]
async fn test_lm_example_unconfigured(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "启用后卸载零残留"
)]
async fn test_lm_example_unload(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "改写参数可见"
)]
async fn test_lm_rewritten_args_visible(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "改写助手先于消息结束"
)]
async fn test_lm_assistant_before_message_end(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-runtime/agent-runtime.feature",
    name = "系统碎片进入压缩上下文"
)]
async fn test_lm_fragment_reaches_fixed_context(agent: AgentState, ws: Workspace) {}
