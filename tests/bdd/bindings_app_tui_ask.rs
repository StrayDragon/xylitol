//! BDD bindings for `app-tui-ask` (c1850).

use crate::tests::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "tui-registers-ask"
)]
fn test_tui_registers_ask(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "print-omits-ask"
)]
fn test_print_omits_ask(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "mounts-choice-slot"
)]
fn test_mounts_choice_slot(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "esc-skips-success"
)]
fn test_esc_skips_success(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "submit-answered"
)]
fn test_submit_answered(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "scrollback-human-rail"
)]
fn test_scrollback_human_rail(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "trust-untouched"
)]
fn test_trust_untouched(ws: Workspace) {
    let _ = ws;
}
