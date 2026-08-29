//! app-tui-bridge BDD 绑定（atb 桥缝模型级族）。

use crate::steps_app_tui_bridge::{BridgeBdd, bridge_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "metadata-events-degrade-quietly-headless"
)]
fn test_atb1_metadata_degrade(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "busy-lifecycle-until-agent-end-headless"
)]
fn test_atb2_busy_lifecycle(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "queue-counts-and-display-diff-headless"
)]
fn test_atb3_queue_and_diff(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "compaction-block-lifecycle-headless"
)]
fn test_atb5_compaction_block(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "auto-retry-status-lifecycle-headless"
)]
fn test_atb6_auto_retry(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "aborted-error-note-dedupe-headless"
)]
fn test_atb7_abort_note_dedupe(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "bash-ui-block-not-notice-headless"
)]
fn test_atb8_bash_block(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "tool-intent-upsert-single-row-headless"
)]
fn test_atb10_tool_intent(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "write-edit-success-quiet-output-headless"
)]
fn test_atb11_write_edit_quiet(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "write-intent-body-before-end-headless"
)]
fn test_atb12_write_intent_body(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "tool-path-stream-sticky-headless"
)]
fn test_atb13_path_sticky(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "error-kind-branch-sticky-vs-abort-headless"
)]
fn test_atb14_error_kinds(bridge_bdd: BridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "wire-roundtrip-preserves-tool-args-headless"
)]
fn test_atb15_wire_parity(bridge_bdd: BridgeBdd) {}
