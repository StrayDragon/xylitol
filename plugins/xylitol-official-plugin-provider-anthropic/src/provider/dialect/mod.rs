use crate::thinking::ResolvedThinking;
use crate::wire_policy::Compat;
use serde_json::{Value, json};

pub fn apply_anthropic_thinking(body: &mut Value, resolved: &ResolvedThinking, compat: Compat) {
    match compat {
        Compat::Generic => crate::thinking::apply_thinking_anthropic(body, resolved),
        Compat::Deepseek => apply_deepseek_anthropic_thinking(body, resolved),
    }
}

fn apply_deepseek_anthropic_thinking(body: &mut Value, resolved: &ResolvedThinking) {
    match resolved {
        ResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("thinking");
            }
        }
        ResolvedThinking::AnthropicBudget(_) => {
            body["thinking"] = json!({ "type": "enabled" });
        }
        ResolvedThinking::OpenAiEffort(_) | ResolvedThinking::Invalid(_) => {}
    }
}
