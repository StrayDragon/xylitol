use crate::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "TUI登记问卷"
)]
fn test_tui_registers_ask(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "print省略问卷"
)]
fn test_print_omits_ask(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "挂载选择槽"
)]
fn test_mounts_choice_slot(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "Esc跳过成功"
)]
fn test_esc_skips_success(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "提交已作答"
)]
fn test_submit_answered(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "回滚人类轨"
)]
fn test_scrollback_human_rail(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-ask/app-tui-ask.feature",
    name = "信任未改"
)]
fn test_trust_untouched(ws: Workspace) {
    let _ = ws;
}

#[scenario(
    path = "llmanspec/specs/app-tui-trust/app-tui-trust.feature",
    name = "信任问卷使用选择提示词于TUI"
)]
fn test_atq_trust_choice_prompt() {}
#[scenario(
    path = "llmanspec/specs/app-tui-trust/app-tui-trust.feature",
    name = "选择槽替换编辑器"
)]
fn test_atq_choice_slot_editor() {}
#[scenario(
    path = "llmanspec/specs/app-tui-trust/app-tui-trust.feature",
    name = "钩子点保持之后信任"
)]
fn test_atq_hook_after_trust(agent: AgentState, ws: Workspace) {}
