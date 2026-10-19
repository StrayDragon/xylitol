//! layer-architecture BDD：只绑产品可观察场景（不扫模块布局）。

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
    name = "server-surface-rpc-entry"
)]
async fn test_la_server_jsonrpc_entry(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "second-writer-rejected-with-conflict"
)]
async fn test_c2826_one_writer(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "protocol-carries-shared-types"
)]
fn test_la_r1520_protocol_shared(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "print-is-cli-submode"
)]
fn test_la_r1531_print_submode(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "runtimes-live-in-infra"
)]
async fn test_la_r1533_runtimes(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "agent-runs-without-session-binding"
)]
async fn test_la_r1534_agent_indep(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "projection-in-agent-render-in-app"
)]
fn test_la_r1521_thin_agent(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "reload-shared-by-server-and-print"
)]
async fn test_la_r1519_reload_same_seam(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "mcp-config-via-embed-seam"
)]
fn test_la_r1518_mcp_seam(t6_mcp_bdd: T6McpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "dispatch-consumed-by-tui"
)]
async fn test_la_r1516_dispatch_consumer(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "session-as-minimal-unit"
)]
async fn test_la_r1513_session_unit(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "provider-diff-contained-in-bridge"
)]
fn test_la_r1528_bridge_open_close(provider_bdd: ProviderBdd) {}
