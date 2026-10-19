//! Thinking level → provider request-body resolution (c1165).

use std::collections::HashMap;

use serde_json::{Value, json};

pub use xylitol_plugin_sdk::{ObsSessionContext, ThinkingBudgets, XyGenerateOptions};

/// Adapter family for resolve (Completions and Responses share effort semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingAdapterKind {
    OpenAi,
    Anthropic,
}

/// Resolved thinking parameter ready for JSON body injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedThinking {
    Omit,
    OpenAiEffort(String),
    AnthropicBudget(u64),
    /// A configured level cannot be represented by Anthropic's token-budget
    /// protocol. Providers turn this into an observable request error.
    Invalid(String),
}

/// Builtin canonical thinking-level names — the single-source request
/// vocabulary (exact spelling, canonical order). Levels stay opaque strings at
/// runtime: this table is consumed for matching/budgets, never as a closed
/// user-facing enum.
pub struct BuiltinThinkingLevels;

impl BuiltinThinkingLevels {
    pub const OFF: &'static str = "off";
    pub const MINIMAL: &'static str = "minimal";
    pub const LOW: &'static str = "low";
    pub const MEDIUM: &'static str = "medium";
    pub const HIGH: &'static str = "high";
    pub const XHIGH: &'static str = "xhigh";
    pub const MAX: &'static str = "max";
    /// All builtin names, canonical order (off first, max last).
    pub const ALL: &'static [&'static str] = &[
        Self::OFF,
        Self::MINIMAL,
        Self::LOW,
        Self::MEDIUM,
        Self::HIGH,
        Self::XHIGH,
        Self::MAX,
    ];
}

/// Return a built-in level only when its configured spelling matches exactly.
fn canonical_known_level(level: &str) -> Option<&'static str> {
    BuiltinThinkingLevels::ALL
        .iter()
        .copied()
        .find(|builtin| *builtin == level)
}

fn builtin_anthropic_budget(level: &str) -> u64 {
    match level {
        BuiltinThinkingLevels::MINIMAL => 1024,
        BuiltinThinkingLevels::LOW => 2048,
        BuiltinThinkingLevels::MEDIUM => 8192,
        BuiltinThinkingLevels::HIGH => 16384,
        BuiltinThinkingLevels::XHIGH | BuiltinThinkingLevels::MAX => 32768,
        _ => unreachable!("budget only requested for canonical known level"),
    }
}

fn budget_for_level(level: &str, budgets: Option<&ThinkingBudgets>) -> u64 {
    let override_budget = budgets.and_then(|b| match level {
        BuiltinThinkingLevels::MINIMAL => b.minimal,
        BuiltinThinkingLevels::LOW => b.low,
        BuiltinThinkingLevels::MEDIUM => b.medium,
        BuiltinThinkingLevels::HIGH => b.high,
        _ => None,
    });
    override_budget.unwrap_or_else(|| builtin_anthropic_budget(level))
}

/// Resolve thinking options into a concrete request-body directive.
pub fn resolve_thinking_for_request(
    level: &str,
    map: &HashMap<String, Option<String>>,
    budgets: Option<&ThinkingBudgets>,
    adapter: ThinkingAdapterKind,
) -> ResolvedThinking {
    // Product support sets, map keys, and known built-in fallbacks all use exact
    // strings. No thinking-level spelling is normalized at this request boundary.
    if let Some(entry) = map.get(level) {
        return match entry {
            None => ResolvedThinking::Omit,
            Some(raw) => match adapter {
                ThinkingAdapterKind::OpenAi => ResolvedThinking::OpenAiEffort(raw.clone()),
                ThinkingAdapterKind::Anthropic => resolve_anthropic_map_string(level, raw, budgets),
            },
        };
    }

    match adapter {
        ThinkingAdapterKind::OpenAi => {
            if canonical_known_level(level) == Some("off") {
                ResolvedThinking::Omit
            } else {
                ResolvedThinking::OpenAiEffort(level.to_string())
            }
        }
        ThinkingAdapterKind::Anthropic => {
            if canonical_known_level(level) == Some("off") {
                ResolvedThinking::Omit
            } else if let Some(known) = canonical_known_level(level) {
                ResolvedThinking::AnthropicBudget(budget_for_level(known, budgets))
            } else {
                ResolvedThinking::Invalid(format!(
                    "Anthropic thinking level `{level}` requires a numeric or known-level thinking_level_map entry"
                ))
            }
        }
    }
}

fn resolve_anthropic_map_string(
    level: &str,
    raw: &str,
    budgets: Option<&ThinkingBudgets>,
) -> ResolvedThinking {
    if let Ok(n) = raw.parse::<u64>() {
        return ResolvedThinking::AnthropicBudget(n);
    }
    let Some(as_level) = canonical_known_level(raw) else {
        return ResolvedThinking::Invalid(format!(
            "Anthropic thinking_level_map `{level}: {raw}` must map to a token budget or known level"
        ));
    };
    if as_level == "off" {
        return ResolvedThinking::Omit;
    }
    ResolvedThinking::AnthropicBudget(budget_for_level(as_level, budgets))
}

/// Inject OpenAI Completions `reasoning_effort` (generic / OpenAI-shaped).
pub fn apply_thinking_openai_completions(body: &mut Value, resolved: &ResolvedThinking) {
    match resolved {
        ResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("reasoning_effort");
            }
        }
        ResolvedThinking::OpenAiEffort(effort) => {
            body["reasoning_effort"] = Value::String(effort.clone());
        }
        ResolvedThinking::AnthropicBudget(_) | ResolvedThinking::Invalid(_) => {}
    }
}

/// Completions thinking inject gated by [`crate::wire_policy::Compat`].
///
/// Prefer [`crate::provider::dialect::apply_completions_thinking`] at call sites;
/// this remains a thin public alias.
pub fn apply_thinking_openai_completions_with_compat(
    body: &mut Value,
    resolved: &ResolvedThinking,
    compat: crate::wire_policy::Compat,
) {
    crate::provider::dialect::apply_completions_thinking(body, resolved, compat);
}

/// Inject OpenAI Responses `reasoning: { effort, summary }` (summary aligns with pi default `auto`).
pub fn apply_thinking_openai_responses(body: &mut Value, resolved: &ResolvedThinking) {
    match resolved {
        ResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("reasoning");
            }
        }
        ResolvedThinking::OpenAiEffort(effort) => {
            body["reasoning"] = json!({ "effort": effort, "summary": "auto" });
        }
        ResolvedThinking::AnthropicBudget(_) | ResolvedThinking::Invalid(_) => {}
    }
}

/// Inject Anthropic Messages `thinking: { type: enabled, budget_tokens }`.
pub fn apply_thinking_anthropic(body: &mut Value, resolved: &ResolvedThinking) {
    match resolved {
        ResolvedThinking::Omit => {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("thinking");
            }
        }
        ResolvedThinking::AnthropicBudget(tokens) => {
            body["thinking"] = json!({
                "type": "enabled",
                "budget_tokens": tokens,
            });
        }
        ResolvedThinking::OpenAiEffort(_) | ResolvedThinking::Invalid(_) => {}
    }
}

/// Resolve options for an adapter kind.
pub fn resolve_from_options(
    options: &XyGenerateOptions,
    adapter: ThinkingAdapterKind,
) -> ResolvedThinking {
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
    fn builtin_levels_table_is_the_canonical_vocabulary() {
        assert_eq!(
            BuiltinThinkingLevels::ALL,
            &["off", "minimal", "low", "medium", "high", "xhigh", "max"]
        );
        for level in BuiltinThinkingLevels::ALL {
            assert_eq!(canonical_known_level(level), Some(*level));
        }
        assert_eq!(canonical_known_level("vendor-max"), None);
        assert_eq!(canonical_known_level("HIGH"), None);
    }

    #[test]
    fn openai_absent_medium_identity() {
        let r = resolve_thinking_for_request(
            "medium",
            &HashMap::new(),
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, ResolvedThinking::OpenAiEffort("medium".into()));
    }

    #[test]
    fn openai_off_omits() {
        let r =
            resolve_thinking_for_request("off", &HashMap::new(), None, ThinkingAdapterKind::OpenAi);
        assert_eq!(r, ResolvedThinking::Omit);
    }

    #[test]
    fn null_map_omits() {
        let mut map = HashMap::new();
        map.insert("high".into(), None);
        let r = resolve_thinking_for_request("high", &map, None, ThinkingAdapterKind::OpenAi);
        assert_eq!(r, ResolvedThinking::Omit);
    }

    #[test]
    fn map_override_high_to_max() {
        let mut map = HashMap::new();
        map.insert("high".into(), Some("max".into()));
        let r = resolve_thinking_for_request("high", &map, None, ThinkingAdapterKind::OpenAi);
        assert_eq!(r, ResolvedThinking::OpenAiEffort("max".into()));
    }

    #[test]
    fn anthropic_budget_defaults() {
        let r = resolve_thinking_for_request(
            "low",
            &HashMap::new(),
            None,
            ThinkingAdapterKind::Anthropic,
        );
        assert_eq!(r, ResolvedThinking::AnthropicBudget(2048));
    }

    #[test]
    fn anthropic_budget_honors_settings_override() {
        let budgets = ThinkingBudgets {
            low: Some(4096),
            ..Default::default()
        };
        let r = resolve_thinking_for_request(
            "low",
            &HashMap::new(),
            Some(&budgets),
            ThinkingAdapterKind::Anthropic,
        );
        assert_eq!(r, ResolvedThinking::AnthropicBudget(4096));
    }

    #[test]
    fn freeform_anthropic_level_is_invalid_without_map() {
        let r = resolve_thinking_for_request(
            "vendor-max",
            &HashMap::new(),
            None,
            ThinkingAdapterKind::Anthropic,
        );
        assert!(matches!(r, ResolvedThinking::Invalid(_)));
    }

    #[test]
    fn openai_preserves_declared_level_spelling() {
        let r = resolve_thinking_for_request(
            "HIGH",
            &HashMap::new(),
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(r, ResolvedThinking::OpenAiEffort("HIGH".into()));
    }

    #[test]
    fn anthropic_known_levels_require_exact_spelling() {
        let high = resolve_thinking_for_request(
            "high",
            &HashMap::new(),
            None,
            ThinkingAdapterKind::Anthropic,
        );
        assert_eq!(high, ResolvedThinking::AnthropicBudget(16_384));

        let off = resolve_thinking_for_request(
            "off",
            &HashMap::new(),
            None,
            ThinkingAdapterKind::Anthropic,
        );
        assert_eq!(off, ResolvedThinking::Omit);

        for level in ["HIGH", " high "] {
            let resolved = resolve_thinking_for_request(
                level,
                &HashMap::new(),
                None,
                ThinkingAdapterKind::Anthropic,
            );
            assert!(
                matches!(resolved, ResolvedThinking::Invalid(_)),
                "{level:?} must not resolve as an Anthropic built-in level"
            );
        }
    }

    #[test]
    fn apply_completions_effort() {
        let mut body = json!({"model": "m"});
        apply_thinking_openai_completions(
            &mut body,
            &ResolvedThinking::OpenAiEffort("medium".into()),
        );
        assert_eq!(body["reasoning_effort"], json!("medium"));
    }

    #[test]
    fn apply_completions_deepseek_thinking_format() {
        let mut body = json!({"model": "m"});
        apply_thinking_openai_completions_with_compat(
            &mut body,
            &ResolvedThinking::OpenAiEffort("medium".into()),
            crate::wire_policy::Compat::Deepseek,
        );
        assert_eq!(body["thinking"]["type"], json!("enabled"));
        assert_eq!(body["reasoning_effort"], json!("medium"));

        apply_thinking_openai_completions_with_compat(
            &mut body,
            &ResolvedThinking::Omit,
            crate::wire_policy::Compat::Deepseek,
        );
        assert_eq!(body["thinking"]["type"], json!("disabled"));
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn apply_responses_effort() {
        let mut body = json!({"model": "m"});
        apply_thinking_openai_responses(&mut body, &ResolvedThinking::OpenAiEffort("high".into()));
        assert_eq!(body["reasoning"]["effort"], json!("high"));
        assert_eq!(body["reasoning"]["summary"], json!("auto"));
    }

    #[test]
    fn apply_anthropic_budget_and_off() {
        let mut body = json!({"model": "m"});
        apply_thinking_anthropic(&mut body, &ResolvedThinking::AnthropicBudget(8192));
        assert_eq!(body["thinking"]["type"], json!("enabled"));
        assert_eq!(body["thinking"]["budget_tokens"], json!(8192));

        apply_thinking_anthropic(&mut body, &ResolvedThinking::Omit);
        assert!(body.get("thinking").is_none());
    }
}
