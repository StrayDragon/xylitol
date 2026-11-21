use crate::bdd::fixtures::*;
use crate::bdd::steps_infra_plugin::{InfraPluginBdd, infra_plugin_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "未批准插件不注册工具"
)]
#[serial_test::serial(env_global)]
async fn test_infra_plugin_unapproved_no_tools(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "批准后注册为 MCP 工具"
)]
#[serial_test::serial(env_global)]
async fn test_infra_plugin_approved_registers_mcp(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {
}

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "停用覆盖批准"
)]
#[serial_test::serial(env_global)]
async fn test_infra_plugin_disabled_overrides_approval(
    ws: Workspace,
    infra_plugin_bdd: InfraPluginBdd,
) {
}

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "未声明mcp不注册服务器"
)]
#[serial_test::serial(env_global)]
async fn test_infra_plugin_no_mcp_declaration(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {}
