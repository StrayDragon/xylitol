//! DeepSeek dialect overlays on native OpenAI / Anthropic APIs.
//!
//! | Native `api` | DeepSeek base / quirk |
//! |---|---|
//! | `openai-responses` | `https://api.deepseek.com` — no `include: reasoning.encrypted_content` (via WirePolicy) |
//! | `openai-completions` | Zen / Completions — `thinking: { type }` (+ optional `reasoning_effort`) |
//! | `anthropic-messages` | `https://api.deepseek.com/anthropic` — thinking enabled; `budget_tokens` ignored by server |

use serde_json::{Value, json};

use crate::thinking::ResolvedThinking;

/// pi `thinkingFormat: deepseek` for Chat Completions.
pub fn apply_completions_thinking(body: &mut Value, resolved: &ResolvedThinking) {
    match resolved {
        ResolvedThinking::Omit => {
            body["thinking"] = json!({ "type": "disabled" });
            if let Some(obj) = body.as_object_mut() {
                obj.remove("reasoning_effort");
            }
        }
        ResolvedThinking::OpenAiEffort(effort) => {
            body["thinking"] = json!({ "type": "enabled" });
            body["reasoning_effort"] = Value::String(effort.clone());
        }
        ResolvedThinking::AnthropicBudget(_) | ResolvedThinking::Invalid(_) => {}
    }
}

/// DeepSeek Anthropic-compatible Messages: `thinking.type` only (`budget_tokens` ignored upstream).
pub fn apply_anthropic_thinking(body: &mut Value, resolved: &ResolvedThinking) {
    match resolved {
        ResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("thinking");
            }
        }
        ResolvedThinking::AnthropicBudget(_) => {
            // Server ignores budget_tokens; still enable thinking without claiming a budget.
            body["thinking"] = json!({ "type": "enabled" });
        }
        ResolvedThinking::OpenAiEffort(_) | ResolvedThinking::Invalid(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_enabled_sets_type_and_effort() {
        let mut body = json!({"model": "m"});
        apply_completions_thinking(&mut body, &ResolvedThinking::OpenAiEffort("medium".into()));
        assert_eq!(body["thinking"]["type"], json!("enabled"));
        assert_eq!(body["reasoning_effort"], json!("medium"));
    }

    #[test]
    fn anthropic_enabled_omits_budget_tokens() {
        let mut body = json!({"model": "m"});
        apply_anthropic_thinking(&mut body, &ResolvedThinking::AnthropicBudget(8192));
        assert_eq!(body["thinking"]["type"], json!("enabled"));
        assert!(
            body["thinking"].get("budget_tokens").is_none(),
            "DeepSeek Anthropic ignores budget_tokens; must not send it: {body}"
        );
    }
}
