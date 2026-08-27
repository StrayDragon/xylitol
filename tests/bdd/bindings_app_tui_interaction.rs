//! app-tui-transcript BDD 绑定（P3：键鼠交互折叠族）。

use crate::steps_app_tui_interaction::{TuiInteraction, tui_interaction};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "tools-per-block-fold-click-and-alt-e-headless"
)]
fn test_att20_tools_per_block(tui_interaction: TuiInteraction) {}

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "thinking-per-id-fold-click-and-ctrl-t-headless"
)]
fn test_att21_thinking_per_id(tui_interaction: TuiInteraction) {}

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "mouse-fold-hit-triangle-column-only-headless"
)]
fn test_att22_triangle_column_only(tui_interaction: TuiInteraction) {}

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "activity-fold-layered-collapse-hides-blocks-headless"
)]
fn test_att25_layered_collapse(tui_interaction: TuiInteraction) {}

#[scenario(
    path = "llmanspec/specs/app-tui-transcript/app-tui-transcript.feature",
    name = "cluster-fold-marker-mouse-precise-headless"
)]
fn test_att31_cluster_precise(tui_interaction: TuiInteraction) {}
