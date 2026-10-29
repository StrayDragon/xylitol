use crate::bdd::fixtures::*;
use crate::bdd::steps_infra_lsp_dap::{LspDapBdd, lsp_dap_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-lsp/infra-lsp.feature",
    name = "未配置LSP零装配"
)]
async fn test_infra_lsp_zero(ws: Workspace, lsp_dap_bdd: LspDapBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-lsp/infra-lsp.feature",
    name = "配置LSP注册工具"
)]
async fn test_infra_lsp_configured(ws: Workspace, lsp_dap_bdd: LspDapBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-dap/infra-dap.feature",
    name = "未配置DAP零装配"
)]
async fn test_infra_dap_zero(ws: Workspace, lsp_dap_bdd: LspDapBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-dap/infra-dap.feature",
    name = "配置DAP注册工具"
)]
async fn test_infra_dap_configured(ws: Workspace, lsp_dap_bdd: LspDapBdd) {}
