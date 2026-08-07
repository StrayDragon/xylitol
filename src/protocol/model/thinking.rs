//! Thinking levels and provider-param resolution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── Thinking Level ──────────────────────────────────────────────────

/// How much "thinking" / chain-of-thought the model should expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
    Max,
}

impl ThinkingLevel {
    /// Default levels when `thinking: true` and no explicit list is configured.
    pub const STANDARD: &'static [ThinkingLevel] = &[
        ThinkingLevel::Off,
        ThinkingLevel::Minimal,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ThinkingLevel::Off => "off",
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
            ThinkingLevel::Xhigh => "xhigh",
            ThinkingLevel::Max => "max",
        }
    }

    /// Parse a level name (case-insensitive). Unknown → `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Some(ThinkingLevel::Off),
            "minimal" => Some(ThinkingLevel::Minimal),
            "low" => Some(ThinkingLevel::Low),
            "medium" => Some(ThinkingLevel::Medium),
            "high" => Some(ThinkingLevel::High),
            "xhigh" => Some(ThinkingLevel::Xhigh),
            "max" => Some(ThinkingLevel::Max),
            _ => None,
        }
    }

    /// Clamp to what the model supports (boolean capability only).
    pub fn clamp(self, model_supports_thinking: bool) -> Self {
        if !model_supports_thinking {
            return ThinkingLevel::Off;
        }
        self
    }

    /// Resolve configured level names into a support list.
    ///
    /// - `thinking == false` → `[Off]`
    /// - `thinking == true` and empty/missing list → [`Self::STANDARD`]
    /// - explicit list → parsed in order (holes allowed); unknown names → `Err`
    pub fn resolve_configured_levels(
        thinking: bool,
        configured: Option<&[String]>,
    ) -> Result<Vec<ThinkingLevel>, String> {
        if !thinking {
            return Ok(vec![ThinkingLevel::Off]);
        }
        let Some(raw) = configured.filter(|v| !v.is_empty()) else {
            return Ok(Self::STANDARD.to_vec());
        };
        let mut out = Vec::with_capacity(raw.len());
        for name in raw {
            let Some(level) = Self::parse(name) else {
                return Err(format!("unknown thinking level: {name}"));
            };
            out.push(level);
        }
        Ok(out)
    }

    /// Highest level in `supported` by full order (`off` … `max`). Empty → `Off`.
    pub fn highest_in(supported: &[ThinkingLevel]) -> ThinkingLevel {
        supported
            .iter()
            .copied()
            .max()
            .unwrap_or(ThinkingLevel::Off)
    }

    /// Adjustable when the support set has any level other than sole `Off`.
    pub fn is_adjustable(supported: &[ThinkingLevel]) -> bool {
        supported.iter().any(|l| *l != ThinkingLevel::Off)
    }

    /// Pick a legal level when the current one is unsupported.
    ///
    /// Prefers `preferred_default` when still legal (session-first assembly only);
    /// otherwise the support-set highest (m10 / c1470). Does **not** fall back to Medium.
    pub fn clamp_to_supported(
        current: ThinkingLevel,
        supported: &[ThinkingLevel],
        preferred_default: Option<ThinkingLevel>,
    ) -> ThinkingLevel {
        if supported.is_empty() {
            return ThinkingLevel::Off;
        }
        if supported.contains(&current) {
            return current;
        }
        if let Some(d) = preferred_default
            && supported.contains(&d)
        {
            return d;
        }
        Self::highest_in(supported)
    }
}

/// Per-model map: ThinkingLevel `as_str` → provider native string, or `null` to omit.
///
/// Missing keys mean adapter built-in defaults (see [`resolve_thinking_for_request`]).
pub type ThinkingLevelMap = HashMap<String, Option<String>>;

/// Optional Settings-style thinking budget overrides (Anthropic budget path).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThinkingBudgets {
    pub minimal: Option<u64>,
    pub low: Option<u64>,
    pub medium: Option<u64>,
    pub high: Option<u64>,
}

/// Adapter family for thinking param resolution (Completions/Responses share effort).
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

/// Validate map keys are known [`ThinkingLevel`] names.
pub fn validate_thinking_level_map(map: &ThinkingLevelMap) -> Result<(), String> {
    for key in map.keys() {
        if ThinkingLevel::parse(key).is_none() {
            return Err(format!("unknown thinking_level_map key: {key}"));
        }
    }
    Ok(())
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

fn budget_for_level(level: ThinkingLevel, budgets: Option<&ThinkingBudgets>) -> u64 {
    let override_budget = budgets.and_then(|b| match level {
        ThinkingLevel::Minimal => b.minimal,
        ThinkingLevel::Low => b.low,
        ThinkingLevel::Medium => b.medium,
        ThinkingLevel::High => b.high,
        // Settings has no xhigh/max slots; fall through to built-in.
        ThinkingLevel::Off | ThinkingLevel::Xhigh | ThinkingLevel::Max => None,
    });
    override_budget.unwrap_or_else(|| builtin_anthropic_budget(level))
}

/// Resolve session thinking level + optional user map into provider params.
///
/// - **Missing map key** → adapter built-in default
/// - **`null` value** → [`ResolvedThinking::Omit`]
/// - **string value** → OpenAI: send as-is; Anthropic: parse as budget u64, or as a
///   level name to look up budget, else built-in for the current level
pub fn resolve_thinking_for_request(
    level: ThinkingLevel,
    map: Option<&ThinkingLevelMap>,
    budgets: Option<&ThinkingBudgets>,
    adapter: ThinkingAdapterKind,
) -> ResolvedThinking {
    if let Some(map) = map
        && let Some(entry) = map.get(level.as_str())
    {
        return match entry {
            None => ResolvedThinking::Omit,
            Some(raw) => match adapter {
                ThinkingAdapterKind::OpenAi => ResolvedThinking::OpenAiEffort(raw.clone()),
                ThinkingAdapterKind::Anthropic => resolve_anthropic_map_string(level, raw, budgets),
            },
        };
    }

    // Key absent → built-in defaults.
    match adapter {
        ThinkingAdapterKind::OpenAi => {
            if level == ThinkingLevel::Off {
                ResolvedThinking::Omit
            } else {
                ResolvedThinking::OpenAiEffort(level.as_str().to_string())
            }
        }
        ThinkingAdapterKind::Anthropic => {
            if level == ThinkingLevel::Off {
                ResolvedThinking::Omit
            } else {
                ResolvedThinking::AnthropicBudget(budget_for_level(level, budgets))
            }
        }
    }
}

fn resolve_anthropic_map_string(
    level: ThinkingLevel,
    raw: &str,
    budgets: Option<&ThinkingBudgets>,
) -> ResolvedThinking {
    if let Ok(n) = raw.parse::<u64>() {
        return ResolvedThinking::AnthropicBudget(n);
    }
    if let Some(as_level) = ThinkingLevel::parse(raw) {
        if as_level == ThinkingLevel::Off {
            return ResolvedThinking::Omit;
        }
        return ResolvedThinking::AnthropicBudget(budget_for_level(as_level, budgets));
    }
    ResolvedThinking::AnthropicBudget(budget_for_level(level, budgets))
}
