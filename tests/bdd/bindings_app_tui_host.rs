//! app-tui-transcript / app-tui-input host-pump BDD 绑定（att9/att11 + ati busy 族）。

use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
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

#[scenario(
    path = "llmanspec/specs/app-tui-commands/app-tui-commands.feature",
    name = "bang-inline-effect-runs-during-bang"
)]
async fn test_atm18_inline_model_during_bang(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "exclusive-stays-queued"
)]
async fn test_ath45_queued_survives_bang(host_pump_bdd: HostPumpBdd) {}

// ── c2835 后继：裸规则回填（app-tui-host 无场景规则 → 可执行示例）──────

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "host-pump-is-the-single-engine-entry"
)]
async fn test_ath1_engine_via_host_pump(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "tui-agents-doc-carries-layout-map"
)]
fn test_ath2_agents_doc_layout_map(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "effects-go-through-the-single-pump"
)]
async fn test_ath4_effects_single_pump(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "resume-list-rows-via-public-seam"
)]
async fn test_ath6_resume_list_seam(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "committed-entries-reuse-row-cache"
)]
fn test_ath34_committed_entries_row_cache(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "streaming-suffix-growth-limits-full-parses"
)]
fn test_ath35_stable_prefix_limits_full_parses(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "debug-build-enables-file-logging-by-default"
)]
fn test_ath9_debug_file_logging_on(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "product-ui-binds-application-owned-at-construction"
)]
fn test_ath30_product_mode_application_owned(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "pty-smoke-registered-and-opt-in"
)]
fn test_ath40_pty_smoke_registration(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "attach-reload-cancel-stops-later-steps"
)]
async fn test_ath41_reload_cooperative_cancel(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "harness-pump-shares-production-effect-branches"
)]
async fn test_ath42_shared_effect_pump(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-host/app-tui-host.feature",
    name = "harness-verifies-without-real-tty"
)]
fn test_ath3_harness_without_tty(host_pump_bdd: HostPumpBdd) {}

// ── c2835 后继：会话树族裸规则回填（宿主侧）────────────────────────

#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "tree-slot-replaces-editor"
)]
async fn test_ats1_tree_slot_replaces_editor(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "live-tree-via-command"
)]
async fn test_ats2_live_tree_via_command(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature",
    name = "tree-reflects-persisted-round"
)]
async fn test_ats6_tree_reflects_persisted(host_pump_bdd: HostPumpBdd) {}
