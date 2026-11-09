use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "产品树分叉创建子项"
)]
fn test_st_tree_fork(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "树槽搜索帮助行"
)]
fn test_st_search_help(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "树标签持久化经Driver"
)]
fn test_st_label(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "调试树夹具非空"
)]
fn test_st_debug_fixture(host_pump_bdd: HostPumpBdd) {}
