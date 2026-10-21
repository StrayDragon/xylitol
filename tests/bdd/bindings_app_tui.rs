use crate::bdd::fixtures::{Workspace, ws};
use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "远端类型保留"
)]
async fn test_atb4_remote_type_kept(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "无未驱动骨架文件"
)]
fn test_at_no_skeleton(ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "表面本地剪贴板与Host资源"
)]
async fn test_at_surface_local(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "产品槽点击跳转"
)]
async fn test_att_product_gutter(host_pump_bdd: HostPumpBdd) {}
