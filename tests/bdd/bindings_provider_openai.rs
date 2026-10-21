use crate::bdd::steps_bridge::{ProviderBdd, provider_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "思考参数按族与兼容"
)]
fn test_c2826_thinking_families(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "Responses错误内嵌消息表面"
)]
fn test_c2826_responses_error(provider_bdd: ProviderBdd) {}
