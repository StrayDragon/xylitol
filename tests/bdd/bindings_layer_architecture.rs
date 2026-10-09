//! layer-architecture BDD：可观察的 server 产品入口（其余分层约束仍为 @human）。

use crate::bdd::fixtures::*;
use crate::bdd::steps_app_tui_bridge::{BridgeBdd, bridge_bdd};
use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_bridge::{AiBridgeBdd, ai_bridge_bdd};
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
    name = "same-process-driver-crosses-layers"
)]
fn test_la_r1525_downward_deps(host_pump_bdd: HostPumpBdd) {}

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
    name = "guarantee-via-docs-and-seam-tests"
)]
fn test_la_r1535_guarantee() {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "projection-in-agent-render-in-app"
)]
fn test_la_r1521_thin_agent(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "single-composition-root"
)]
async fn test_la_r1522_composition(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "xy-ports-in-protocol"
)]
fn test_la_r1523_xy_ports(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "seam-separated-from-surfaces"
)]
async fn test_la_r1524_seam_split(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "shared-dispatch-across-surfaces"
)]
async fn test_la_r1526_shared_dispatch(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "no-dead-wrapper-on-seam"
)]
async fn test_la_r1527_no_dead_wrapper(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "embed-entry-without-reach-in"
)]
fn test_la_r1517_embed_entry() {}

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
    name = "optional-flags-prefixed-builtins-plain"
)]
fn test_la_r1529_feature_flags() {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "default-features-minimal-quartet"
)]
fn test_la_r1530_default_features() {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "pi-doc-refs-cleared"
)]
fn test_la_r1507_pi_refs() {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "one-canonical-type-per-concept"
)]
fn test_la_r1508_canonical_types() {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "curated-reexport-list"
)]
fn test_la_r1509_reexports() {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "schemars-at-infra-config-edge"
)]
fn test_la_r12_schemars_edge() {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "two-roles-one-process"
)]
async fn test_la_r1510_two_roles(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "session-as-minimal-unit"
)]
async fn test_la_r1513_session_unit(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "provider-diff-contained-in-bridge"
)]
fn test_la_r1528_bridge_open_close(ai_bridge_bdd: AiBridgeBdd) {}
