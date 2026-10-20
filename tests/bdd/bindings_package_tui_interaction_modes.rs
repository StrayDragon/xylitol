//! BDD bindings for `package-tui-interaction-modes` (c2070).

use crate::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "缺省模式行内2"
)]
fn test_default_inline(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "复制时发布OSC52"
)]
fn test_copy_on_release_osc52(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "停靠栏拖选钳制保留选区"
)]
fn test_dock_drag_clamp_keeps_selection(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "滚轮粘滞视口"
)]
fn test_wheel_sticky_viewport(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "复制提示之后成功"
)]
fn test_copy_notice_after_success(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "crates/xylitol-tui/llmanspec/specs/package-tui-interaction-modes/package-tui-interaction-modes.feature",
    name = "编辑器多行选区"
)]
fn test_editor_multiline_selection(ws: Workspace) {
    let _ = ws;
}
