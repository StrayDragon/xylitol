//! BDD bindings for `package-tui-scrollbar-gutter` (c2891).

use crate::bdd::steps_package_tui_scrollbar_gutter::{GutterBdd, gutter_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "gutter-stamped-on-right-edge"
)]
fn test_gutter_stamped_on_right_edge(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "gutter-hidden-when-no-overflow"
)]
fn test_gutter_hidden_when_no_overflow(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "top-click-scrolls-to-head"
)]
fn test_top_click_scrolls_to_head(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "bottom-click-rearms-follow"
)]
fn test_bottom_click_rearms_follow(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "gutter-down-never-starts-selection"
)]
fn test_gutter_down_never_starts_selection(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "thumb-reflect-scroll-ratio"
)]
fn test_thumb_reflect_scroll_ratio(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "track-press-jumps-proportionally"
)]
fn test_track_press_jumps_proportionally(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}

#[scenario(
    path = "packages/xylitol-tui/llmanspec/specs/package-tui-scrollbar-gutter/package-tui-scrollbar-gutter.feature",
    name = "drag-follows-pointer-until-release"
)]
fn test_drag_follows_pointer_until_release(gutter_bdd: GutterBdd) {
    let _ = gutter_bdd;
}
