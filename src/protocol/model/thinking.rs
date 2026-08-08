//! Thinking-level configuration and common selection helpers.

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
///
/// Any entry other than case-insensitive `off` counts as adjustable, including
/// freeform vendor literals outside the legacy presentation enum.
pub fn thinking_levels_are_adjustable(levels: &[String]) -> bool {
    levels
        .iter()
        .any(|level| !level.trim().eq_ignore_ascii_case(THINKING_OFF))
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
