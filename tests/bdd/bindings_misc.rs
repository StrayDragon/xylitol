use crate::bdd::fixtures::*;
use crate::bdd::steps_bridge::{PromptBdd, ProviderBdd, prompt_bdd, provider_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "接线观察矩阵"
)]
fn test_hooks_wiring_observe(agent: AgentState, event: String, op: String, context_key: String) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "接线取消矩阵"
)]
fn test_hooks_wiring_cancel(agent: AgentState, event: String, op: String, block_reason: String) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "未知操作"
)]
async fn test_hooks_wiring_unknown_op(agent: AgentState) {}
#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "Responses工具调用流之前完成"
)]
fn test_pab13_responses_toolcall_stream(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "部分参数对象"
)]
fn test_pab14_partial_args(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "Responses系统作为developer"
)]
fn test_pab15_system_developer(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "Responses思考不于输出文本"
)]
fn test_pab15_thinking_omit(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "Responses正文存储严格摘要包含"
)]
fn test_pab16_body_fields(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "Responses推理项设置思考签名"
)]
fn test_pab16_thinking_signature(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "收集工具指南"
)]
fn test_pt9_collect(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "自定义提示词无静默工具回填"
)]
fn test_pt9_no_backfill(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "无斜杠提示词模板"
)]
fn test_pt3_no_slash_prompt_templates(prompt_bdd: PromptBdd, ws: Workspace) {}

#[test]
fn curated_xy_hook_bus_symbols_resolve() {
    fn assert_port<T: ?Sized>() {}
    assert_port::<dyn xylitol::XyHookBus>();
    let _ = xylitol::XyHookOutcome::Allowed;
    let _ = xylitol::NoopHookBus;
}
