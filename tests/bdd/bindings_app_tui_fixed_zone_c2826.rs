//! app-tui-fixed-zone c2826 specs-compact 场景绑定。

use crate::tests::bdd::steps_app_tui_fixed_zone::{FixedZoneBdd, fixed_zone_bdd};
use crate::tests::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "idle-status-empty-busy-single-line"
)]
fn test_c2826_atc_idle_busy(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "footer-single-line-with-heuristic-provenance"
)]
async fn test_c2826_atc_footer_heuristic(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "idle-editor-zone-keeps-compact-border"
)]
fn test_c2826_atc_editor_compact(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "busy-spinner-single-row-with-lead"
)]
fn test_c2826_atc_spinner(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "user-message-keeps-prefix-no-wash"
)]
fn test_c2826_atc_user_prefix(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "compacting-status-stays-single-row"
)]
fn test_c2826_atc_compacting(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "abort-returns-status-to-idle"
)]
async fn test_c2826_atc_abort_idle(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "idle-no-ready-wording"
)]
fn test_c2826_atc_no_ready(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "unknown-theme-keeps-old-and-diagnoses"
)]
async fn test_c2826_atc_theme_diag(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "theme-slot-esc-closes-without-change"
)]
async fn test_c2826_atc_theme_esc(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "thinking-level-shown-in-footer"
)]
fn test_c2826_atc_thinking_footer(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "loaded-resources-card-renders-skills-and-mcp"
)]
fn test_c2826_atc_loaded_resources(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "footer-derived-percent-from-api-usage"
)]
async fn test_c2826_atc_footer_percent(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "toast-single-slot-replaces-with-error-prefix"
)]
fn test_c2826_atc_toast(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "footer-used-compact-k-notation"
)]
async fn test_c2826_atc_footer_compact(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "reload-status-lead-reloading"
)]
async fn test_c2826_atc_reload_status(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "tool-header-timeout-note-shown-when-requested"
)]
fn test_c2826_atc_tool_timeout(fixed_zone_bdd: FixedZoneBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "models-picker-footer-reflects-active"
)]
async fn test_c2826_atc_models_footer(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "footer-token-refresh-single-estimate"
)]
async fn test_c2826_atc_footer_refresh(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature",
    name = "short-terminal-busy-lead-visible"
)]
fn test_c2826_atc_short_terminal(host_pump_bdd: HostPumpBdd) {}
