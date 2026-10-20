use crate::bdd::fixtures::{AgentState, Workspace, agent, ws};
use crate::bdd::steps_cli_surface::{
    AttachBdd, CliEntryBdd, CliHelpBdd, SurfaceBdd, SurfaceFlagsBdd, attach_bdd, cli_entry_bdd,
    cli_help_bdd, surface_bdd, surface_flags_bdd,
};
use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "表面TUI动词"
)]
fn test_ce16_tui(surface_bdd: SurfaceBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "表面print动词"
)]
fn test_ce16_print(surface_bdd: SurfaceBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "操作留在顶层"
)]
fn test_ce16_ops(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "服务操作动词"
)]
fn test_ce8_serve(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "表面开关时TUI"
)]
fn test_ce19_tui_flags(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "表面开关时TUI运行"
)]
fn test_ce19_tui_run(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "表面开关TUIattach"
)]
fn test_ce19_tui_attach(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "表面开关时print"
)]
fn test_ce19_print(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "表面开关时print信任"
)]
fn test_ce19_print_trust(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "表面开关时print无信任"
)]
fn test_ce19_print_no_trust(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "顶层表面开关拒绝"
)]
fn test_ce19_toplevel_reject(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "续跑提示当已持久化"
)]
fn test_ce20_hint_yes(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "续跑提示缺席当未持久化"
)]
fn test_ce20_hint_no(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "TUIattachHost向下"
)]
fn test_ce21_attach_down(attach_bdd: AttachBdd) {}

// ---- c2826 specs-compact：新增裸规则场景绑定 ----

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "旧版短名称不已解析"
)]
fn test_c2826_legacy_names(cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "内置表产品名称"
)]
fn test_c2826_builtin_table(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "空闲斜杠已解析进入待处理命令"
)]
fn test_c2826_idle_slash_pending(cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "不支持树种类显式错误"
)]
async fn test_c2826_tree_kind(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "损坏配置硬失败"
)]
fn test_c2826_broken_config(ws: Workspace, cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "零模型配置硬失败"
)]
fn test_c2826_zero_models(ws: Workspace, cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "未设模型显示作为不设置"
)]
fn test_c2826_unset_display(cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "CLI为独立表面"
)]
fn test_ce_surface_independent(cli_help_bdd: CliHelpBdd) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "应用表面到达agent经Driver缝"
)]
fn test_ce_driver_seam() {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "组合构建端口来自基建"
)]
fn test_ce_ports_from_infra(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "CLI与服务端共享一引导"
)]
fn test_ce_one_bootstrap(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "运行复用引导会话标识"
)]
fn test_ce_reuse_session_id(surface_flags_bdd: SurfaceFlagsBdd) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "未持久化会话无续跑提示"
)]
fn test_ce_unpersisted_no_hint(surface_flags_bdd: SurfaceFlagsBdd) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "已配置缺省显示"
)]
async fn test_ce_c2841_configured_default_shown(agent: AgentState, cli_entry_bdd: CliEntryBdd) {}
