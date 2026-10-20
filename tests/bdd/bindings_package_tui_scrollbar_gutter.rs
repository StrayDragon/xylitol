//! BDD bindings for `package-tui-scrollbar-gutter` (c2891).

use crate::bdd::steps_package_tui_scrollbar_gutter::{GutterBdd, gutter_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "槽盖章时右边缘"
)]
fn test_gutter_stamped_on_right_edge(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "槽隐藏当无溢出"
)]
fn test_gutter_hidden_when_no_overflow(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "顶部点击滚动至头部"
)]
fn test_top_click_scrolls_to_head(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "底部点击再武装跟随"
)]
fn test_bottom_click_rearms_follow(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "槽向下从不启动选区"
)]
fn test_gutter_down_never_starts_selection(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "滑块反映滚动比率"
)]
fn test_thumb_reflect_scroll_ratio(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "轨道按下跳转按比例"
)]
fn test_track_press_jumps_proportionally(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "拖选跟随指针直到发布"
)]
fn test_drag_follows_pointer_until_release(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}
