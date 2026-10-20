//! app-tui-preview BDD 绑定（c2930）。

use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_cli_surface::{CliHelpBdd, cli_help_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "调试TUI帮助列出designing"
)]
fn test_preview_debug_tui_help(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "发布TUI帮助省略designing"
)]
fn test_preview_release_tui_help(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "帮助省略顶层designing"
)]
fn test_preview_toplevel_help(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "designing会不需要Host监听"
)]
fn test_preview_designing_no_attach(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "designing目录列出场景"
)]
fn test_preview_catalog(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "选择场景注入画布"
)]
async fn test_preview_select_injects(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "选择另一场景切换画布"
)]
async fn test_designing_switch_scene(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "闭合场景槽重开"
)]
async fn test_designing_slot_reopens(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "未知场景错误无崩溃"
)]
async fn test_preview_unknown_scene(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "designing注入缺席来自线协议命令"
)]
fn test_preview_not_on_wire() {}
