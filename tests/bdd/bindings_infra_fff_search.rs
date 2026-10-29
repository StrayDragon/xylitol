use crate::bdd::steps_fff_search::{FffSearchBdd, fff_search_bdd};
use crate::bdd::steps_tool_selection::{ToolSelectionBdd, tool_selection_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-fff-search.feature",
    name = "ffgrep 内容检索"
)]
async fn test_c2997_ffgrep_content(fff_search_bdd: FffSearchBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-fff-search.feature",
    name = "fffind 路径检索"
)]
async fn test_c2997_fffind_path(fff_search_bdd: FffSearchBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-fff-search.feature",
    name = "ffgrep 拒绝零超时"
)]
async fn test_c2997_ffgrep_zero_timeout(fff_search_bdd: FffSearchBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-fff-search.feature",
    name = "索引按工作区隔离"
)]
async fn test_c2997_index_isolated(fff_search_bdd: FffSearchBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-fff-search.feature",
    name = "插件缺失回落内置"
)]
fn test_c2997_fff_missing(tool_selection_bdd: ToolSelectionBdd) {}
