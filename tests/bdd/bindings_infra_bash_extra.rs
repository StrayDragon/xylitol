use crate::bdd::fixtures::{AgentState, Workspace, XySessionStore, agent, sess, ws};
use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "bang中止取消执行器"
)]
async fn test_ib_abort(agent: AgentState, ws: Workspace, host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "bang执行已记录作为消息"
)]
async fn test_ib_record(sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "空闲bang路由至执行器"
)]
async fn test_ib_route(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "bash之前钩子拒绝"
)]
async fn test_ib_hook_reject(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "排除bash不于上下文"
)]
fn test_ib_exclude() {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "回显经执行器"
)]
fn test_bs_r1429_echo(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "溢出至临时文件"
)]
fn test_bs_r1431_truncate(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "trait抽象可mock"
)]
fn test_bs_r1426_trait(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "递增超时杀死"
)]
fn test_bs_r1428_timeout(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "中止可达来自运行时"
)]
async fn test_bs_r1435_abort(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-bash/infra-bash.feature",
    name = "可选超时与缺失参数"
)]
fn test_bs_r1436_missing_arg(ws: Workspace) {}
