use crate::fixtures::*;
use crate::steps_bridge::{AiBridgeBdd, PromptBdd, ai_bridge_bdd, prompt_bdd};
use crate::steps_tokenizer::{TokenizerBdd, tokenizer_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "session-start"
)]
async fn test_hooks_wiring_session_start(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "model-select"
)]
async fn test_hooks_wiring_model_select(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "thinking-select"
)]
async fn test_hooks_wiring_thinking_select(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "session-tree"
)]
async fn test_hooks_wiring_session_tree(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "tree-cancel"
)]
async fn test_hooks_wiring_tree_cancel(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "session-shutdown"
)]
async fn test_hooks_wiring_session_shutdown(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "switch-cancel"
)]
async fn test_hooks_wiring_switch_cancel(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "user-bash"
)]
async fn test_hooks_wiring_user_bash(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "user-bash-block"
)]
async fn test_hooks_wiring_user_bash_block(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/test-hooks-wiring/test-hooks-wiring.feature",
    name = "unknown-op"
)]
async fn test_hooks_wiring_unknown_op(agent: AgentState) {}
#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-toolcall-streams-before-done"
)]
fn test_pab13_responses_toolcall_stream(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "partial-args-object"
)]
fn test_pab14_partial_args(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-system-as-developer"
)]
fn test_pab15_system_developer(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-thinking-not-in-output-text"
)]
fn test_pab15_thinking_omit(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-body-store-strict-summary-include"
)]
fn test_pab16_body_fields(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-reasoning-item-sets-thinking-signature"
)]
fn test_pab16_thinking_signature(ai_bridge_bdd: AiBridgeBdd) {}

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

#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "cache-list-and-remove"
)]
fn test_paa8_list_remove(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "download-atomic"
)]
fn test_paa8_atomic(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "hf-endpoint-mirror"
)]
fn test_paa9_mirror(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/package-ai-bridge-accounting/package-ai-bridge-accounting.feature",
    name = "hf-endpoint-default"
)]
fn test_paa9_default(tokenizer_bdd: TokenizerBdd) {}

#[test]
fn curated_xy_hook_bus_symbols_resolve() {
    fn assert_port<T: ?Sized>() {}
    assert_port::<dyn xylitol::XyHookBus>();
    let _ = xylitol::XyHookOutcome::Allowed;
    let _ = xylitol::NoopHookBus;
}
