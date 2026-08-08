//! Thinking level → provider request-body resolution (c1165).

use std::collections::HashMap;

use serde_json::{Value, json};

/// Optional Settings-style budget overrides for Anthropic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiBridgeThinkingBudgets {
    pub minimal: Option<u64>,
    pub low: Option<u64>,
    pub medium: Option<u64>,
    pub high: Option<u64>,
}

/// Options carried into [`crate::provider::AiBridgeLlmAdapter::generate_stream`].
#[derive(Debug, Clone)]
pub struct AiBridgeGenerateOptions {
    /// Level name (`off`, `medium`, …) matching domain `ThinkingLevel::as_str`.
    pub thinking_level: String,
    pub level_map: HashMap<String, Option<String>>,
    pub thinking_budgets: Option<AiBridgeThinkingBudgets>,
    /// System prompt injected each request via the adapter's formal channel
    /// (Responses `developer`/`system`, Completions system message, Anthropic `system`).
    pub system_prompt: Option<String>,
}

impl Default for AiBridgeGenerateOptions {
    fn default() -> Self {
        Self {
            thinking_level: "off".into(),
            level_map: HashMap::new(),
            thinking_budgets: None,
            system_prompt: None,
        }
    }
}

/// Adapter family for resolve (Completions and Responses share effort semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBridgeThinkingAdapterKind {
    OpenAi,
    Anthropic,
}

/// Resolved thinking parameter ready for JSON body injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiBridgeResolvedThinking {
    Omit,
    OpenAiEffort(String),
    AnthropicBudget(u64),
    /// A configured level cannot be represented by Anthropic's token-budget
    /// protocol. Providers turn this into an observable request error.
    Invalid(String),
}

fn canonical_known_level(level: &str) -> Option<&'static str> {
    match level.trim().to_ascii_lowercase().as_str() {
        "off" => Some("off"),
        "minimal" => Some("minimal"),
        "low" => Some("low"),
        "medium" => Some("medium"),
        "high" => Some("high"),
        "xhigh" => Some("xhigh"),
        "max" => Some("max"),
        _ => None,
    }
}

fn builtin_anthropic_budget(level: &str) -> u64 {
    match level {
        "minimal" => 1024,
        "low" => 2048,
        "medium" => 8192,
        "high" => 16384,
        "xhigh" | "max" => 32768,
        _ => unreachable!("budget only requested for canonical known level"),
    }
}

fn budget_for_level(level: &str, budgets: Option<&AiBridgeThinkingBudgets>) -> u64 {
    let override_budget = budgets.and_then(|b| match level {
        "minimal" => b.minimal,
        "low" => b.low,
        "medium" => b.medium,
        "high" => b.high,
        _ => None,
    });
    override_budget.unwrap_or_else(|| builtin_anthropic_budget(level))
}

/// Resolve thinking options into a concrete request-body directive.
pub fn resolve_thinking_for_request(
    level: &str,
    map: &HashMap<String, Option<String>>,
    budgets: Option<&AiBridgeThinkingBudgets>,
    adapter: AiBridgeThinkingAdapterKind,
) -> AiBridgeResolvedThinking {
    // Map keys are declared level names, so preserve the configured spelling
    // for lookup and OpenAI effort values.
    if let Some(entry) = map.get(level) {
        return match entry {
            None => AiBridgeResolvedThinking::Omit,
            Some(raw) => match adapter {
                AiBridgeThinkingAdapterKind::OpenAi => {
                    AiBridgeResolvedThinking::OpenAiEffort(raw.clone())
                }
                AiBridgeThinkingAdapterKind::Anthropic => {
                    resolve_anthropic_map_string(level, raw, budgets)
                }
            },
        };
    }

    match adapter {
        AiBridgeThinkingAdapterKind::OpenAi => {
            if canonical_known_level(level) == Some("off") {
                AiBridgeResolvedThinking::Omit
            } else {
                AiBridgeResolvedThinking::OpenAiEffort(level.to_string())
            }
        }
        AiBridgeThinkingAdapterKind::Anthropic => {
            if canonical_known_level(level) == Some("off") {
                AiBridgeResolvedThinking::Omit
            } else if let Some(known) = canonical_known_level(level) {
                AiBridgeResolvedThinking::AnthropicBudget(budget_for_level(known, budgets))
            } else {
                AiBridgeResolvedThinking::Invalid(format!(
                    "Anthropic thinking level `{level}` requires a numeric or known-level thinking_level_map entry"
                ))
            }
        }
    }
}

fn resolve_anthropic_map_string(
    level: &str,
    raw: &str,
    budgets: Option<&AiBridgeThinkingBudgets>,
) -> AiBridgeResolvedThinking {
    if let Ok(n) = raw.parse::<u64>() {
        return AiBridgeResolvedThinking::AnthropicBudget(n);
    }
    let Some(as_level) = canonical_known_level(raw) else {
        return AiBridgeResolvedThinking::Invalid(format!(
            "Anthropic thinking_level_map `{level}: {raw}` must map to a token budget or known level"
        ));
    };
    if as_level == "off" {
        return AiBridgeResolvedThinking::Omit;
    }
    AiBridgeResolvedThinking::AnthropicBudget(budget_for_level(as_level, budgets))
}

/// Inject OpenAI Completions `reasoning_effort` (generic / OpenAI-shaped).
pub fn apply_thinking_openai_completions(body: &mut Value, resolved: &AiBridgeResolvedThinking) {
    match resolved {
        AiBridgeResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("reasoning_effort");
            }
        }
        AiBridgeResolvedThinking::OpenAiEffort(effort) => {
            body["reasoning_effort"] = Value::String(effort.clone());
        }
        AiBridgeResolvedThinking::AnthropicBudget(_) | AiBridgeResolvedThinking::Invalid(_) => {}
    }
}

/// Completions thinking inject gated by [`crate::wire_policy::Compat`].
///
/// Prefer [`crate::provider::dialect::apply_completions_thinking`] at call sites;
/// this remains a thin public alias.
pub fn apply_thinking_openai_completions_with_compat(
    body: &mut Value,
    resolved: &AiBridgeResolvedThinking,
    compat: crate::wire_policy::Compat,
) {
    crate::provider::dialect::apply_completions_thinking(body, resolved, compat);
}

/// Anthropic thinking inject gated by compat (DeepSeek omits `budget_tokens`).
pub fn apply_thinking_anthropic_with_compat(
    body: &mut Value,
    resolved: &AiBridgeResolvedThinking,
    compat: crate::wire_policy::Compat,
) {
    crate::provider::dialect::apply_anthropic_thinking(body, resolved, compat);
}

/// Inject OpenAI Responses `reasoning: { effort, summary }` (summary aligns with pi default `auto`).
pub fn apply_thinking_openai_responses(body: &mut Value, resolved: &AiBridgeResolvedThinking) {
    match resolved {
        AiBridgeResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("reasoning");
            }
        }
        AiBridgeResolvedThinking::OpenAiEffort(effort) => {
            body["reasoning"] = json!({ "effort": effort, "summary": "auto" });
        }
        AiBridgeResolvedThinking::AnthropicBudget(_) | AiBridgeResolvedThinking::Invalid(_) => {}
    }
}

/// Inject Anthropic Messages `thinking: { type: enabled, budget_tokens }`.
pub fn apply_thinking_anthropic(body: &mut Value, resolved: &AiBridgeResolvedThinking) {
    match resolved {
        AiBridgeResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("thinking");
            }
        }
        AiBridgeResolvedThinking::AnthropicBudget(tokens) => {
            body["thinking"] = json!({
                "type": "enabled",
                "budget_tokens": tokens,
            });
        }
        AiBridgeResolvedThinking::OpenAiEffort(_) | AiBridgeResolvedThinking::Invalid(_) => {}
    }
}

/// Resolve options for an adapter kind.
pub fn resolve_from_options(
    options: &AiBridgeGenerateOptions,
    adapter: AiBridgeThinkingAdapterKind,
) -> AiBridgeResolvedThinking {
    resolve_thinking_for_request(
        &options.thinking_level,
        &options.level_map,
        options.thinking_budgets.as_ref(),
        adapter,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_absent_medium_identity() {
        let r = resolve_thinking_for_request(
            "medium",
            &HashMap::new(),
            None,
            AiBridgeThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, AiBridgeResolvedThinking::OpenAiEffort("medium".into()));
    }

    #[test]
    fn openai_off_omits() {
        let r = resolve_thinking_for_request(
            "off",
            &HashMap::new(),
            None,
            AiBridgeThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, AiBridgeResolvedThinking::Omit);
    }

    #[test]
    fn null_map_omits() {
        let mut map = HashMap::new();
        map.insert("high".into(), None);
        let r =
            resolve_thinking_for_request("high", &map, None, AiBridgeThinkingAdapterKind::OpenAi);
        assert_eq!(r, AiBridgeResolvedThinking::Omit);
    }

    #[test]
    fn map_override_high_to_max() {
        let mut map = HashMap::new();
        map.insert("high".into(), Some("max".into()));
        let r =
            resolve_thinking_for_request("high", &map, None, AiBridgeThinkingAdapterKind::OpenAi);
        assert_eq!(r, AiBridgeResolvedThinking::OpenAiEffort("max".into()));
    }

    #[test]
    fn anthropic_budget_defaults() {
        let r = resolve_thinking_for_request(
            "low",
            &HashMap::new(),
            None,
            AiBridgeThinkingAdapterKind::Anthropic,
        );
        assert_eq!(r, AiBridgeResolvedThinking::AnthropicBudget(2048));
    }

    #[test]
    fn freeform_anthropic_level_is_invalid_without_map() {
        let r = resolve_thinking_for_request(
            "vendor-max",
            &HashMap::new(),
            None,
            AiBridgeThinkingAdapterKind::Anthropic,
        );
        assert!(matches!(r, AiBridgeResolvedThinking::Invalid(_)));
    }

    #[test]
    fn openai_preserves_declared_level_spelling() {
        let r = resolve_thinking_for_request(
            "HIGH",
            &HashMap::new(),
            None,
            AiBridgeThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, AiBridgeResolvedThinking::OpenAiEffort("HIGH".into()));
    }

    #[test]
    fn anthropic_known_level_parsing_is_case_insensitive() {
        let r = resolve_thinking_for_request(
            "HIGH",
            &HashMap::new(),
            None,
            AiBridgeThinkingAdapterKind::Anthropic,
        );
        assert_eq!(r, AiBridgeResolvedThinking::AnthropicBudget(16_384));
    }

    #[test]
    fn apply_completions_effort() {
        let mut body = json!({"model": "m"});
        apply_thinking_openai_completions(
            &mut body,
            &AiBridgeResolvedThinking::OpenAiEffort("medium".into()),
        );
        assert_eq!(body["reasoning_effort"], json!("medium"));
    }

    #[test]
    fn apply_completions_deepseek_thinking_format() {
        let mut body = json!({"model": "m"});
        apply_thinking_openai_completions_with_compat(
            &mut body,
            &AiBridgeResolvedThinking::OpenAiEffort("medium".into()),
            crate::wire_policy::Compat::Deepseek,
        );
        assert_eq!(body["thinking"]["type"], json!("enabled"));
        assert_eq!(body["reasoning_effort"], json!("medium"));

        apply_thinking_openai_completions_with_compat(
            &mut body,
            &AiBridgeResolvedThinking::Omit,
            crate::wire_policy::Compat::Deepseek,
        );
        assert_eq!(body["thinking"]["type"], json!("disabled"));
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn apply_responses_effort() {
        let mut body = json!({"model": "m"});
        apply_thinking_openai_responses(
            &mut body,
            &AiBridgeResolvedThinking::OpenAiEffort("high".into()),
        );
        assert_eq!(body["reasoning"]["effort"], json!("high"));
        assert_eq!(body["reasoning"]["summary"], json!("auto"));
    }

    #[test]
    fn apply_anthropic_budget_and_off() {
        let mut body = json!({"model": "m"});
        apply_thinking_anthropic(&mut body, &AiBridgeResolvedThinking::AnthropicBudget(8192));
        assert_eq!(body["thinking"]["type"], json!("enabled"));
        assert_eq!(body["thinking"]["budget_tokens"], json!(8192));

        apply_thinking_anthropic(&mut body, &AiBridgeResolvedThinking::Omit);
        assert!(body.get("thinking").is_none());
    }
}
