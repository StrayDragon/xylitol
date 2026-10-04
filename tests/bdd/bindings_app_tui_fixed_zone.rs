//! app-tui-fixed-zone BDD — 待办栏下缘堆叠（r21）。

use crate::tests::bdd::steps_app_tui_fixed_zone::{FixedZoneBdd, fixed_zone_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "todo-bar-sits-between-queue-and-toast"
)]
fn test_todo_bar_sits_between_queue_and_toast(fixed_zone_bdd: FixedZoneBdd) {}

// ── c2835 后继：attach 头卡由下行快照驱动 ────────────────────────────

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "resource-header-refreshed-from-snapshot"
)]
fn test_ath38_header_from_downlink(fixed_zone_bdd: FixedZoneBdd) {}
