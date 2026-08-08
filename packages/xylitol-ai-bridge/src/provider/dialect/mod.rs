//! Layer-2 **dialect** adjustments on top of [`super::native`] first-language APIs.
//!
//! YAML `models.*.compat` selects a named profile → [`crate::wire_policy::WirePolicy`]
//! plus request-body tweaks (thinking field shapes, include gates, …).
//!
//! Add a new provider dialect as a sibling module (e.g. `qwen.rs`); do **not** fork
//! native adapters per gateway.

pub mod deepseek;

use crate::thinking::AiBridgeResolvedThinking;
use crate::wire_policy::Compat;
use serde_json::Value;

/// Apply Completions thinking fields for the active compat profile.
pub fn apply_completions_thinking(
    body: &mut Value,
    resolved: &AiBridgeResolvedThinking,
    compat: Compat,
) {
    match compat {
        Compat::Generic => crate::thinking::apply_thinking_openai_completions(body, resolved),
        Compat::Deepseek => deepseek::apply_completions_thinking(body, resolved),
    }
}

/// Apply Anthropic Messages thinking fields for the active compat profile.
pub fn apply_anthropic_thinking(
    body: &mut Value,
    resolved: &AiBridgeResolvedThinking,
    compat: Compat,
) {
    match compat {
        Compat::Generic => crate::thinking::apply_thinking_anthropic(body, resolved),
        Compat::Deepseek => deepseek::apply_anthropic_thinking(body, resolved),
    }
}
