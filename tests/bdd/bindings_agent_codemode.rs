use crate::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "exposure 决定模型与 codemode 可见面"
)]
fn test_exposure_surfaces(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "tool-exposure 拒绝的工具不进入 catalog"
)]
fn test_tool_exposure_catalog(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "deferred 工具不内联"
)]
fn test_deferred_declarations(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "嵌套调用复用权限且拒绝递归"
)]
fn test_nested_dispatch(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "fuel 耗尽与取消映射为错误"
)]
fn test_fuel_exhaustion(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "未配置 codemode 零装配"
)]
fn test_zero_assembly(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "session 工具进入声明与 catalog"
)]
fn test_session_catalog_declarations(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "禁用工具不出现在声明"
)]
fn test_disabled_tool_declarations(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "tool-exposure 过滤同时作用于声明"
)]
fn test_tool_exposure_declarations(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "enabled 禁用 run_code 合法"
)]
fn test_enabled_disabled_run_code(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "未启用但选择 run_code 告警"
)]
fn test_disabled_selected_run_code(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "首次调用才创建 runner"
)]
fn test_lazy_runner(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-codemode/agent-codemode.feature",
    name = "空 catalog 声明为空"
)]
fn test_empty_catalog_declarations(ws: Workspace) {}
