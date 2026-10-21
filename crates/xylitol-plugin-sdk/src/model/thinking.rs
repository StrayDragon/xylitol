use std::collections::HashMap;

pub const THINKING_OFF: &str = "off";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ThinkingConfigError {
    #[error("thinking_levels[{index}] must not be empty")]
    EmptyLevel { index: usize },
    #[error("thinking_level_map key `{key}` is not declared in thinking_levels")]
    UndeclaredMapKey { key: String },
}

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThinkingBudgets {
    pub minimal: Option<u64>,
    pub low: Option<u64>,
    pub medium: Option<u64>,
    pub high: Option<u64>,
}
