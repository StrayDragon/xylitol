use std::collections::HashMap;

pub const THINKING_OFF: &str = "off";

pub const CANONICAL_THINKING_LEVELS: &[&str] =
    &["off", "minimal", "low", "medium", "high", "xhigh", "max"];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ThinkingConfigError {
    #[error("thinking_level_map key must not be empty")]
    EmptyLevel,
    #[error("thinking_level_map[`{level}`] must be null or a non-empty string")]
    EmptyWireValue { level: String },
}

pub fn resolve_levels_from_map(
    thinking: bool,
    map: &ThinkingLevelMap,
) -> Result<Vec<String>, ThinkingConfigError> {
    validate_thinking_level_map(map)?;
    if !thinking || map.is_empty() {
        return Ok(vec![THINKING_OFF.into()]);
    }
    Ok(ordered_levels(map))
}

fn canonical_rank(level: &str) -> Option<usize> {
    CANONICAL_THINKING_LEVELS
        .iter()
        .position(|known| *known == level)
}

fn ordered_levels(map: &ThinkingLevelMap) -> Vec<String> {
    let mut levels: Vec<&String> = map.keys().collect();
    levels.sort_by(|a, b| match (canonical_rank(a), canonical_rank(b)) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.cmp(b),
    });
    levels.into_iter().cloned().collect()
}

pub fn thinking_levels_are_adjustable(levels: &[String]) -> bool {
    levels.iter().any(|level| level != THINKING_OFF)
}

pub fn last_declared_thinking_level(levels: &[String]) -> String {
    levels
        .last()
        .cloned()
        .unwrap_or_else(|| THINKING_OFF.into())
}

pub type ThinkingLevelMap = HashMap<String, Option<String>>;

pub fn validate_thinking_level_map(map: &ThinkingLevelMap) -> Result<(), ThinkingConfigError> {
    for (level, wire) in map {
        if level.trim().is_empty() {
            return Err(ThinkingConfigError::EmptyLevel);
        }
        if wire.as_ref().is_some_and(|wire| wire.trim().is_empty()) {
            return Err(ThinkingConfigError::EmptyWireValue {
                level: level.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThinkingBudgets {
    pub minimal: Option<u64>,
    pub low: Option<u64>,
    pub medium: Option<u64>,
    pub high: Option<u64>,
}
