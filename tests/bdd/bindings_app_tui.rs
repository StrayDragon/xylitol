//! BDD bindings for the app-tui-bridge carrier contract (atb4 remote-type-kept).
//! Product attach: HTTP POST /rpc plus HttpWsClient unary on WS /rpc.

use crate::bdd::fixtures::{Workspace, ws};
use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "remote-type-kept"
)]
async fn test_atb4_remote_type_kept(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "no-undriven-skeleton-files"
)]
fn test_at_no_skeleton(ws: Workspace) {}

// r1162 面本地 vs host 角色分界（c2922 补绑）
#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "surface-local-clipboard-and-host-resources"
)]
async fn test_at_surface_local(host_pump_bdd: HostPumpBdd) {}

// r2893 产品 AO 滚动槽（c2922 补绑，步骤复用包侧语义）
#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "product-gutter-click-jumps"
)]
async fn test_att_product_gutter(host_pump_bdd: HostPumpBdd) {}
