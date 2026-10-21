use crate::bdd::fixtures::*;
use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/workspace-assembly/workspace-assembly.feature",
    name = "扫描工作区仅装配适配层声明 rutis"
)]
fn test_wa_rutis_dep_only_at_adapter() {}

#[scenario(
    path = "llmanspec/specs/workspace-assembly/workspace-assembly.feature",
    name = "空 mcp 列表不含 mcp 前缀工具"
)]
fn test_wa_unconfigured_mcp_registers_no_tools() {}

#[scenario(
    path = "llmanspec/specs/workspace-assembly/workspace-assembly.feature",
    name = "空 hooks 分发结果为允许"
)]
async fn test_wa_unconfigured_hooks_start_no_process() {}

#[scenario(
    path = "llmanspec/specs/workspace-assembly/workspace-assembly.feature",
    name = "host.describe 宣告 jsonrpc"
)]
async fn test_wa_envelope_describe_declares_jsonrpc(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/workspace-assembly/workspace-assembly.feature",
    name = "无会话无工具仍跑完一轮"
)]
async fn test_wa_runtime_loop_runs_without_session(agent: AgentState, ws: Workspace) {}
