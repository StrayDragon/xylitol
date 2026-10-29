use crate::bdd::fixtures::*;
use crate::bdd::steps_infra_plugin::{InfraPluginBdd, infra_plugin_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "受信外部插件装配工具"
)]
async fn test_infra_plugin_trusted(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "未受信外部插件不装配"
)]
async fn test_infra_plugin_untrusted(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "保留名外部插件不装配"
)]
async fn test_infra_plugin_reserved(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "外部插件握手与调用"
)]
async fn test_infra_plugin_handshake(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "协议版本不匹配拒绝"
)]
async fn test_infra_plugin_protocol_mismatch(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-plugin/infra-plugin.feature",
    name = "失败插件不阻断装配"
)]
async fn test_infra_plugin_failure_isolation(ws: Workspace, infra_plugin_bdd: InfraPluginBdd) {}
