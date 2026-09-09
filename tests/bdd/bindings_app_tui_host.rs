//! app-tui-transcript / app-tui-input host-pump BDD 绑定（att9/att11 + ati busy 族）。

use crate::tests::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "product-bash-block-lifecycle-headless"
)]
async fn test_att9_bang_lifecycle(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "bang-block-rail-no-wash-headless"
)]
async fn test_att11_bang_rail(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "busy-abort-and-idle-quit-keys-headless"
)]
async fn test_ati2_busy_abort_idle_quit(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "busy-enter-steer-alt-enter-followup-headless"
)]
async fn test_ati3_steer_followup(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "busy-esc-aborts-clears-steer-no-tree-headless"
)]
async fn test_ati10_busy_esc_wiring(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "abort-then-resubmit-runs-again-headless"
)]
async fn test_ati14_abort_resumable(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "idle-bang-execute-not-run-headless"
)]
async fn test_ati16_idle_bang(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "bang-esc-aborts-hanging-bash-headless"
)]
async fn test_ati19_bang_esc_aborts(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "bash-active-second-bang-hard-reject-headless"
)]
async fn test_ati20_second_bang_reject(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "slash-session-tree-pending-not-prompt-headless"
)]
async fn test_ati28_slash_session_tree(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "bang-esc-then-second-bang-still-abortable-headless"
)]
async fn test_ati30_second_bang_abortable(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "abort-latch-suppresses-late-xy-headless"
)]
async fn test_ati31_late_xy_suppressed(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "agent-busy-bang-prefix-hard-reject-headless"
)]
async fn test_ati32_busy_bang_prefix(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "reload-soft-gate-keys-headless"
)]
async fn test_ati43_reload_soft_gate(host_pump_bdd: HostPumpBdd) {}
