use crate::bdd::steps_cli_print_extra::{T4PrintBdd, t4_print_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "print流增量仅"
)]
async fn test_cp_delta(t4_print_bdd: T4PrintBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "print显示工具名称与摘要"
)]
async fn test_cp_tool(t4_print_bdd: T4PrintBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "print无累积载荷"
)]
async fn test_cp_dedup(t4_print_bdd: T4PrintBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "print错误非零工具失败零"
)]
async fn test_cp_error(t4_print_bdd: T4PrintBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-print/cli-print.feature",
    name = "思考流至stderr仅"
)]
fn test_cp_thinking_stderr(t4_print_bdd: T4PrintBdd) {}
