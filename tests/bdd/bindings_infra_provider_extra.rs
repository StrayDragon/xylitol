use crate::bdd::steps_agent_tools_extra::{T2SchemaBdd, t2_schema_bdd};
use crate::bdd::steps_bridge::{ProviderBdd, provider_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "输入项携带类型"
)]
fn test_ip_items_typed(t2_schema_bdd: T2SchemaBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "单一层适配器包裹"
)]
fn test_ip_r1497_single_wrap() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "Responses适配器请求正文"
)]
fn test_ip_r1498_responses_body(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "Anthropic消息思考分支"
)]
fn test_ip_r1502_anthropic_branch(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "适配器选区按种类与API"
)]
fn test_ip_r1503_selection() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "单一Xy模型装配路径"
)]
fn test_ip_r1505_single_path() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "厂商类型留在于桥缝"
)]
fn test_ip_r1506_vendor_boundary() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "模型端口接受桥缝DTO"
)]
fn test_ip_r1499_bridge_dto() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "具名兼容配置档注入"
)]
fn test_ip_r1500_named_compat(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "API字面量完整名称"
)]
fn test_ip_r1501_full_names() {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "OpenAI用户图片投影"
)]
fn test_ip_r2975_user_image(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-provider/infra-provider.feature",
    name = "OpenAI工具结果图片投影"
)]
fn test_ip_r2975_tool_image(provider_bdd: ProviderBdd) {}
