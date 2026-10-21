use crate::bdd::fixtures::*;
use crate::bdd::steps_app_tui_bridge::{BridgeBdd, bridge_bdd};
use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_bridge::{ProviderBdd, provider_bdd};
use crate::bdd::steps_c2827::{T6McpBdd, t6_mcp_bdd};
use crate::bdd::steps_cli_surface::{SurfaceFlagsBdd, surface_flags_bdd};
use crate::bdd::steps_protocol::{ProtocolBdd, protocol_bdd};
use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "服务端表面RPC入口"
)]
async fn test_la_server_jsonrpc_entry(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "第二次写者拒绝带冲突"
)]
async fn test_c2826_one_writer(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "协议携带共享类型"
)]
fn test_la_r1520_protocol_shared(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "print为CLI子模式"
)]
fn test_la_r1531_print_submode(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "运行时在线于基建"
)]
async fn test_la_r1533_runtimes(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "agent运行无会话绑定"
)]
async fn test_la_r1534_agent_indep(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "投影于agent渲染于应用"
)]
fn test_la_r1521_thin_agent(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "重载共享按服务端与print"
)]
async fn test_la_r1519_reload_same_seam(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "MCP配置经嵌入缝"
)]
fn test_la_r1518_mcp_seam(t6_mcp_bdd: T6McpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "分发被消费按TUI"
)]
async fn test_la_r1516_dispatch_consumer(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "会话作为最小单测"
)]
async fn test_la_r1513_session_unit(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "厂商diff收纳于桥缝"
)]
fn test_la_r1528_bridge_open_close(provider_bdd: ProviderBdd) {}
