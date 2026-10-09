//! app-tui-input c2826 specs-compact 场景绑定。

use crate::bdd::steps_app_tui_fixed_zone::{FixedZoneBdd, fixed_zone_bdd};
use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "selector-replaces-editor-slot-not-content-top"
)]
async fn test_c2826_ati_selector(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "double-esc-opens-tree-esc-restores-editor"
)]
async fn test_c2826_ati_double_esc(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "queue-strip-renders-and-alt-up-restores"
)]
async fn test_c2826_ati_queue_strip(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "same-text-twice-two-user-bubbles"
)]
fn test_c2826_ati_same_text(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "send-history-recall-with-arrow-up"
)]
async fn test_c2826_ati_send_history(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "bang-prefix-switches-editor-border"
)]
fn test_c2826_ati_bang_border(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "tree-slot-driven-by-live-driver-tree"
)]
async fn test_c2826_ati_live_tree(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "paste-image-stages-tempfile-path"
)]
async fn test_c2826_ati_paste_image(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "abort-flushes-partial-with-footnote"
)]
async fn test_c2826_ati_abort_partial(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "attach-zero-depth-keeps-local-strip"
)]
async fn test_c2826_ati_zero_depth(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "ctrl-g-stub-without-editor-env"
)]
async fn test_c2826_ati_ctrl_g(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "model-picker-slot-esc-no-change"
)]
async fn test_c2826_ati_picker_esc(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-input/app-tui-input.feature",
    name = "keybindings-reload-failure-keeps-old"
)]
fn test_c2826_ati_keybindings(host_pump_bdd: HostPumpBdd) {}
