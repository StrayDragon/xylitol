use crate::bdd::fixtures::*;
use crate::bdd::steps_bridge::{PromptBdd, ProviderBdd, prompt_bdd, provider_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "wiring-observe-matrix"
)]
fn test_hooks_wiring_observe(agent: AgentState, event: String, op: String, context_key: String) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "wiring-cancel-matrix"
)]
fn test_hooks_wiring_cancel(agent: AgentState, event: String, op: String, block_reason: String) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "unknown-op"
)]
async fn test_hooks_wiring_unknown_op(agent: AgentState) {}
#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "responses-toolcall-streams-before-done"
)]
fn test_pab13_responses_toolcall_stream(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "partial-args-object"
)]
fn test_pab14_partial_args(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "responses-system-as-developer"
)]
fn test_pab15_system_developer(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "responses-thinking-not-in-output-text"
)]
fn test_pab15_thinking_omit(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "responses-body-store-strict-summary-include"
)]
fn test_pab16_body_fields(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "responses-reasoning-item-sets-thinking-signature"
)]
fn test_pab16_thinking_signature(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "collect-tool-guidelines"
)]
fn test_pt9_collect(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "custom-prompt-no-silent-tools-backfill"
)]
fn test_pt9_no_backfill(prompt_bdd: PromptBdd) {}

#[scenario(
    path = "llmanspec/specs/agent-prompt/agent-prompt.feature",
    name = "no-slash-prompt-templates"
)]
fn test_pt3_no_slash_prompt_templates(prompt_bdd: PromptBdd, ws: Workspace) {}

#[test]
fn curated_xy_hook_bus_symbols_resolve() {
    fn assert_port<T: ?Sized>() {}
    assert_port::<dyn xylitol::XyHookBus>();
    let _ = xylitol::XyHookOutcome::Allowed;
    let _ = xylitol::NoopHookBus;
}
