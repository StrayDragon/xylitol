//! app-tui-preview BDD 绑定（c2930）。

use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_cli_surface::{CliHelpBdd, cli_help_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "debug-tui-help-lists-designing"
)]
fn test_preview_debug_tui_help(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "release-tui-help-omits-designing"
)]
fn test_preview_release_tui_help(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "help-omits-toplevel-designing"
)]
fn test_preview_toplevel_help(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "designing-does-not-need-host-listener"
)]
fn test_preview_designing_no_attach(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "designing-catalog-lists-scenes"
)]
fn test_preview_catalog(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "selecting-scene-injects-canvas"
)]
async fn test_preview_select_injects(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "selecting-another-scene-switches-canvas"
)]
async fn test_designing_switch_scene(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "closed-scene-slot-reopens"
)]
async fn test_designing_slot_reopens(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "unknown-scene-errors-without-crash"
)]
async fn test_preview_unknown_scene(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-preview/app-tui-preview.feature",
    name = "designing-inject-absent-from-wire-commands"
)]
fn test_preview_not_on_wire() {}
