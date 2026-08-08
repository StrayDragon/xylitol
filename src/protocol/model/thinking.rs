//! Thinking-level configuration and provider parameter resolution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The literal which disables thinking for every adapter.
pub const THINKING_OFF: &str = "off";

/// Known legacy names used only for presentation and known Anthropic budgets.
///
/// Runtime support sets intentionally use `String`: configured vendors may
/// declare names outside this helper enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    #[default]
    Off,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl ThinkingLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => THINKING_OFF,
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }

    /// Parse a known display/palette name (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            THINKING_OFF => Some(Self::Off),
            "minimal" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" => Some(Self::Xhigh),
            "max" => Some(Self::Max),
            _ => None,
        }
    }
}

/// Resolve a model's declared support list without imposing a global enum.
///
/// `thinking: false`, omitted `thinking_levels`, and an explicitly empty list
/// all produce the non-adjustable `[off]` set.
pub fn resolve_configured_levels(
    thinking: bool,
    configured: Option<&[String]>,
) -> Result<Vec<String>, String> {
    if let Some(raw) = configured {
        for (index, level) in raw.iter().enumerate() {
            if level.trim().is_empty() {
                return Err(format!("thinking_levels[{index}] must not be empty"));
            }
        }
    }

    if !thinking {
        return Ok(vec![THINKING_OFF.into()]);
    }
    let Some(raw) = configured.filter(|levels| !levels.is_empty()) else {
        return Ok(vec![THINKING_OFF.into()]);
    };
    Ok(raw.to_vec())
}

/// Whether a declared list exposes an adjustable thinking level.
pub fn thinking_levels_are_adjustable(levels: &[String]) -> bool {
    levels
        .iter()
        .any(|level| ThinkingLevel::parse(level) != Some(ThinkingLevel::Off))
}

/// Selection default: the final configured entry, never an ordinal "highest".
pub fn last_declared_thinking_level(levels: &[String]) -> String {
    levels
        .last()
        .cloned()
        .unwrap_or_else(|| THINKING_OFF.into())
}

/// Per-model map: declared level name → provider native value, or `null` to omit.
pub type ThinkingLevelMap = HashMap<String, Option<String>>;

/// Validate that every mapping key is part of this model's declared list.
pub fn validate_thinking_level_map(
    map: &ThinkingLevelMap,
    declared_levels: &[String],
) -> Result<(), String> {
    for key in map.keys() {
        if !declared_levels.iter().any(|level| level == key) {
            return Err(format!(
                "thinking_level_map key `{key}` is not declared in thinking_levels"
            ));
        }
    }
    Ok(())
}

/// Optional Settings-style thinking budget overrides (Anthropic budget path).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThinkingBudgets {
    pub minimal: Option<u64>,
    pub low: Option<u64>,
    pub medium: Option<u64>,
    pub high: Option<u64>,
}

/// Adapter family for thinking parameter resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingAdapterKind {
    OpenAi,
    Anthropic,
}

/// Resolved thinking parameter ready for request-body injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedThinking {
    /// Do not send thinking/effort fields.
    Omit,
    /// OpenAI Completions `reasoning_effort` / Responses `reasoning.effort`.
    OpenAiEffort(String),
    /// Anthropic `thinking: { type: "enabled", budget_tokens }`.
    AnthropicBudget(u64),
}

fn builtin_anthropic_budget(level: ThinkingLevel) -> u64 {
    match level {
        ThinkingLevel::Off => 0,
        ThinkingLevel::Minimal => 1024,
        ThinkingLevel::Low => 2048,
        ThinkingLevel::Medium => 8192,
        ThinkingLevel::High => 16384,
        ThinkingLevel::Xhigh | ThinkingLevel::Max => 32768,
    }
}

fn budget_for_known_level(level: ThinkingLevel, budgets: Option<&ThinkingBudgets>) -> u64 {
    let override_budget = budgets.and_then(|b| match level {
        ThinkingLevel::Minimal => b.minimal,
        ThinkingLevel::Low => b.low,
        ThinkingLevel::Medium => b.medium,
        ThinkingLevel::High => b.high,
        ThinkingLevel::Off | ThinkingLevel::Xhigh | ThinkingLevel::Max => None,
    });
    override_budget.unwrap_or_else(|| builtin_anthropic_budget(level))
}

/// Resolve a session's exact level string plus optional map into provider params.
///
/// A freeform non-`off` level must have an Anthropic mapping. Falling back to
/// the historic `medium` budget would misrepresent the configured vendor knob.
pub fn resolve_thinking_for_request(
    level: &str,
    map: Option<&ThinkingLevelMap>,
    budgets: Option<&ThinkingBudgets>,
    adapter: ThinkingAdapterKind,
) -> Result<ResolvedThinking, String> {
    if let Some(map) = map
        && let Some(entry) = map.get(level)
    {
        return match entry {
            None => Ok(ResolvedThinking::Omit),
            Some(raw) => match adapter {
                ThinkingAdapterKind::OpenAi => Ok(ResolvedThinking::OpenAiEffort(raw.clone())),
                ThinkingAdapterKind::Anthropic => resolve_anthropic_map_string(level, raw, budgets),
            },
        };
    }

    match adapter {
        ThinkingAdapterKind::OpenAi if ThinkingLevel::parse(level) == Some(ThinkingLevel::Off) => {
            Ok(ResolvedThinking::Omit)
        }
        ThinkingAdapterKind::OpenAi => Ok(ResolvedThinking::OpenAiEffort(level.to_string())),
        ThinkingAdapterKind::Anthropic
            if ThinkingLevel::parse(level) == Some(ThinkingLevel::Off) =>
        {
            Ok(ResolvedThinking::Omit)
        }
        ThinkingAdapterKind::Anthropic => {
            let known = ThinkingLevel::parse(level).ok_or_else(|| {
                format!(
                    "Anthropic thinking level `{level}` requires a numeric or known-level thinking_level_map entry"
                )
            })?;
            Ok(ResolvedThinking::AnthropicBudget(budget_for_known_level(
                known, budgets,
            )))
        }
    }
}

fn resolve_anthropic_map_string(
    level: &str,
    raw: &str,
    budgets: Option<&ThinkingBudgets>,
) -> Result<ResolvedThinking, String> {
    if let Ok(tokens) = raw.parse::<u64>() {
        return Ok(ResolvedThinking::AnthropicBudget(tokens));
    }
    let mapped = ThinkingLevel::parse(raw).ok_or_else(|| {
        format!(
            "Anthropic thinking_level_map `{level}: {raw}` must map to a token budget or known level"
        )
    })?;
    if mapped == ThinkingLevel::Off {
        return Ok(ResolvedThinking::Omit);
    }
    Ok(ResolvedThinking::AnthropicBudget(budget_for_known_level(
        mapped, budgets,
    )))
}
