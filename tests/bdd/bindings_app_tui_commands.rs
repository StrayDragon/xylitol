//! app-tui-commands BDD 绑定（c2826 specs-compact 斜杠族）。

use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_cli_surface::{CliEntryBdd, cli_entry_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "slash-exit-requests-quit"
)]
async fn test_c2826_slash_exit(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "slash-model-opens-list-and-unknown-errors"
)]
async fn test_c2826_slash_model(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "slash-model-with-arg-direct-set"
)]
async fn test_c2826_model_arg(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "slash-session-name-via-shared-dispatch"
)]
async fn test_c2826_session_name(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "debug-slash-lists-and-rejects-colon"
)]
async fn test_c2826_debug(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "slash-session-tree-opens-tree"
)]
async fn test_c2826_tree(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "session-io-dispatch-by-suffix"
)]
async fn test_c2826_session_io(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "session-info-dump-as-system-text"
)]
async fn test_c2826_session_info(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "session-resume-opens-panel-via-seam"
)]
async fn test_c2826_resume(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "session-lifecycle-verbs-via-seam"
)]
async fn test_c2826_lifecycle(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "reload-idle-fires-busy-rejects"
)]
async fn test_c2826_reload(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "trust-persists-via-driver-seam"
)]
async fn test_c2826_trust(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "history-copy-last-copies-assistant-text"
)]
async fn test_c2826_copy_last(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "theme-slot-and-direct-apply"
)]
async fn test_c2826_theme(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "busy-slash-allow-and-reject"
)]
async fn test_c2826_busy_policy(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "mcp-panel-opens-from-cached-snapshot"
)]
async fn test_c2826_mcp(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "mvp-needs-model-and-exit"
)]
fn test_atc_mvp_two_commands(cli_entry_bdd: CliEntryBdd) {}
#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "single-slash-parser-in-commands-module"
)]
fn test_atc_single_parser(cli_entry_bdd: CliEntryBdd) {}
