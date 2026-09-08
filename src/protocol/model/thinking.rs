//! Thinking-level configuration and common selection helpers.

use std::collections::HashMap;

/// The literal which disables thinking for every adapter.
pub const THINKING_OFF: &str = "off";

/// Failures while resolving a model's freeform thinking config (not `Xy*`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ThinkingConfigError {
    #[error("thinking_levels[{index}] must not be empty")]
    EmptyLevel { index: usize },
    #[error("thinking_level_map key `{key}` is not declared in thinking_levels")]
    UndeclaredMapKey { key: String },
}

/// Resolve a model's declared support list without imposing a global enum.
///
/// `thinking: false`, omitted `thinking_levels`, and an explicitly empty list
/// all produce the non-adjustable `[off]` set.
pub fn resolve_configured_levels(
    thinking: bool,
    configured: Option<&[String]>,
) -> Result<Vec<String>, ThinkingConfigError> {
    if let Some(raw) = configured {
        for (index, level) in raw.iter().enumerate() {
            if level.trim().is_empty() {
                return Err(ThinkingConfigError::EmptyLevel { index });
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
/// Any entry other than exact `off` counts as adjustable, including freeform
/// vendor literals outside the legacy presentation enum.
pub fn thinking_levels_are_adjustable(levels: &[String]) -> bool {
    levels.iter().any(|level| level != THINKING_OFF)
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
) -> Result<(), ThinkingConfigError> {
    for key in map.keys() {
        if !declared_levels.iter().any(|level| level == key) {
            return Err(ThinkingConfigError::UndeclaredMapKey { key: key.clone() });
        }
    }
    Ok(())
}

/// Optional Settings-style thinking budget overrides (Anthropic budget path).
pub use xylitol_ai_bridge::AiBridgeThinkingBudgets as ThinkingBudgets;

/// Builtin canonical thinking-level names (bridge request vocabulary, exact
/// spelling; NOT a closed runtime enum — declared lists stay opaque strings).
pub use xylitol_ai_bridge::AiBridgeBuiltinThinkingLevels as BuiltinThinkingLevels;
