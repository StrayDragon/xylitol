//! package-ai-bridge BDD 绑定。

use crate::bdd::steps_bridge::{AiBridgeBdd, ai_bridge_bdd};
use rstest_bdd_macros::scenario;

// ---- c2826 specs-compact：裸规则转场景绑定 ----

#[scenario(
    path = "packages/xylitol-ai-bridge/llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "thinking-params-by-family-and-compat"
)]
fn test_c2826_thinking_families(ai_bridge_bdd: AiBridgeBdd) {}

#[scenario(
    path = "packages/xylitol-ai-bridge/llmanspec/specs/package-ai-bridge/package-ai-bridge.feature",
    name = "responses-error-embedded-message-surface"
)]
fn test_c2826_responses_error(ai_bridge_bdd: AiBridgeBdd) {}
