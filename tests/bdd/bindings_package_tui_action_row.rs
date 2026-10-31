use crate::bdd::steps_package_tui_action_row::{ActionRowBdd, action_row_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-action-row/package-tui-action-row.feature",
    name = "区间落在宽度内且互不重叠"
)]
fn test_action_row_ranges_within_width(action_row_bdd: ActionRowBdd) {
    let _ = action_row_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-action-row/package-tui-action-row.feature",
    name = "禁用 chip 不进区间表"
)]
fn test_action_row_disabled_chip_unmapped(action_row_bdd: ActionRowBdd) {
    let _ = action_row_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-action-row/package-tui-action-row.feature",
    name = "宽度不足从尾丢弃整 chip"
)]
fn test_action_row_narrow_width_drops_tail(action_row_bdd: ActionRowBdd) {
    let _ = action_row_bdd;
}
