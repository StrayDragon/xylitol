//! provider-openai BDD 绑定。

use crate::bdd::steps_bridge::{ProviderBdd, provider_bdd};
use rstest_bdd_macros::scenario;

// ---- c2826 specs-compact：裸规则转场景绑定 ----

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "thinking-params-by-family-and-compat"
)]
fn test_c2826_thinking_families(provider_bdd: ProviderBdd) {}

#[scenario(
    path = "plugins/xylitol-official-plugin-provider-openai/llmanspec/specs/provider-openai/provider-openai.feature",
    name = "responses-error-embedded-message-surface"
)]
fn test_c2826_responses_error(provider_bdd: ProviderBdd) {}
