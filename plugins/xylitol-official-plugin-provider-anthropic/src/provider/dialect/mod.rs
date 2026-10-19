//! Dialect overlays on Anthropic Messages (DeepSeek compat).

use crate::thinking::AiBridgeResolvedThinking;
use crate::wire_policy::Compat;
use serde_json::{Value, json};

/// Apply Anthropic Messages thinking fields for the active compat profile.
pub fn apply_anthropic_thinking(
    body: &mut Value,
    resolved: &AiBridgeResolvedThinking,
    compat: Compat,
) {
    match compat {
        Compat::Generic => crate::thinking::apply_thinking_anthropic(body, resolved),
        Compat::Deepseek => apply_deepseek_anthropic_thinking(body, resolved),
    }
}

fn apply_deepseek_anthropic_thinking(body: &mut Value, resolved: &AiBridgeResolvedThinking) {
    match resolved {
        AiBridgeResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("thinking");
            }
        }
        AiBridgeResolvedThinking::AnthropicBudget(_) => {
            body["thinking"] = json!({ "type": "enabled" });
        }
        AiBridgeResolvedThinking::OpenAiEffort(_) | AiBridgeResolvedThinking::Invalid(_) => {}
    }
}
