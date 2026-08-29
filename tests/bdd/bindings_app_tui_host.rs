//! app-tui-transcript / app-tui-input host-pump BDD 绑定（att9/att11 起步）。

use crate::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
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
