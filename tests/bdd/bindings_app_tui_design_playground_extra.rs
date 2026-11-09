use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_app_tui_transcript::{TranscriptBdd, transcript_bdd};
use crate::bdd::steps_package_tui_tree_selector::{TreeSelBdd, tree_sel_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "词元同步单一脚本"
)]
fn test_pg_r1206_token_sync() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "Markdown正文已样式化无字面标记"
)]
fn test_pg_r1207_markdown_body(transcript_bdd: TranscriptBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "TUIagent阅读顺序"
)]
fn test_pg_r1208_reading_order() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "种类前缀主题化不写死角色"
)]
fn test_pg_r1209_kind_prefix(tree_sel_bdd: TreeSelBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "在线树来自命令"
)]
async fn test_pg_r1210_live_tree(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "静态槽样例覆盖"
)]
fn test_pg_r1213_static_slots() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "词元校验于门禁"
)]
fn test_pg_r1211_lint_gate() {}

#[scenario(
    path = "llmanspec/specs/app-tui-design-playground/app-tui-design-playground.feature",
    name = "TUIagent名称检查命令"
)]
fn test_pg_r1212_lint_docs() {}
