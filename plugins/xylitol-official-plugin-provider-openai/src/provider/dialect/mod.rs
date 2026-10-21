pub mod deepseek;

use crate::thinking::ResolvedThinking;
use crate::wire_policy::Compat;
use serde_json::Value;

pub fn apply_completions_thinking(body: &mut Value, resolved: &ResolvedThinking, compat: Compat) {
    match compat {
        Compat::Generic => crate::thinking::apply_thinking_openai_completions(body, resolved),
        Compat::Deepseek => deepseek::apply_completions_thinking(body, resolved),
    }
}

pub fn apply_anthropic_thinking(body: &mut Value, resolved: &ResolvedThinking, compat: Compat) {
    match compat {
        Compat::Generic => crate::thinking::apply_thinking_anthropic(body, resolved),
        Compat::Deepseek => deepseek::apply_anthropic_thinking(body, resolved),
    }
}
