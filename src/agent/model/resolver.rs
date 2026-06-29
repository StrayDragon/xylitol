//! ModelResolver — pattern-based model resolution.
//!
//! Provides:
//! - Exact match: `provider/modelId` or bare `id`
//! - Fuzzy match: partial id or name substring match
//! - Alias preference: shorter/cleaner ids over dated versions
//! - `model:thinkingLevel` suffix parsing
//! - Fallback model construction

use crate::domain::model::ModelConfig;
#[cfg(test)]
use crate::domain::model::ModelKind;
use crate::domain::types::{ModelMeta, ThinkingLevel};

// ── Resolved Model ──────────────────────────────────────────────────

/// Result of resolving a model pattern against the available models.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedModel {
    /// The resolved model metadata.
    pub(crate) model: ModelMeta,
    /// Optional thinking level parsed from `model:level` suffix.
    pub(crate) thinking_level: Option<ThinkingLevel>,
    /// Warning message, e.g., when using a fallback.
    pub(crate) warning: Option<String>,
}

impl ResolvedModel {
    pub(crate) fn new(model: ModelMeta) -> Self {
        Self {
            model,
            thinking_level: None,
            warning: None,
        }
    }

    pub(crate) fn with_warning(mut self, warning: String) -> Self {
        self.warning = Some(warning);
        self
    }
}

// ── Resolution ──────────────────────────────────────────────────────

/// Resolve a model by pattern string against available models.
///
/// Resolution order:
/// 1. Parse `model:thinkingLevel` suffix from pattern
/// 2. Try exact match: `provider/modelId` form
/// 3. Try bare id exact match
/// 4. Try fuzzy match (partial id or display_name substring)
/// 5. Prefer aliases (no version suffix) over dated versions
/// 6. Fallback to first available model
pub(crate) fn resolve_model(
    pattern: &str,
    available: &[&ModelMeta],
    default_provider: Option<&str>,
) -> Result<ResolvedModel, String> {
    if available.is_empty() {
        return Err("no models available".to_string());
    }

    // Step 1: Parse thinking level suffix: "model:level"
    let (model_pattern, thinking_level) = parse_thinking_suffix(pattern);

    // Step 2: Try exact match by "provider/modelId"
    if let Some(found) = exact_match_provider_model(&model_pattern, available) {
        return Ok(ResolvedModel::new(found.clone()).with_thinking_level_opt(thinking_level));
    }

    // Step 3: Try exact match by bare id
    if let Some(found) = exact_match_bare_id(&model_pattern, available) {
        return Ok(ResolvedModel::new(found.clone()).with_thinking_level_opt(thinking_level));
    }

    // Step 4: Try fuzzy match (substring in id or display_name)
    if let Some(found) = fuzzy_match(&model_pattern, available) {
        return Ok(ResolvedModel::new(found.clone())
            .with_thinking_level_opt(thinking_level)
            .with_warning(format!(
                "Fuzzy-matched model '{}' for pattern '{}'",
                found.id, model_pattern
            )));
    }

    // Step 5: Build fallback model
    if let Some(fallback) = build_fallback_model(&model_pattern, available, default_provider) {
        return Ok(ResolvedModel::new(fallback)
            .with_thinking_level_opt(thinking_level)
            .with_warning(format!(
                "Model '{}' not found. Using fallback.",
                model_pattern
            )));
    }

    // Step 6: Last resort — first available
    let first = available.first().expect("non-empty: checked above");
    Ok(ResolvedModel::new((*first).clone())
        .with_thinking_level_opt(thinking_level)
        .with_warning(format!(
            "Model '{}' not found. Using first available: {}",
            model_pattern, first.id
        )))
}

// ── Thinking Level Parsing ──────────────────────────────────────────

/// Parse `model:thinkingLevel` suffix.
///
/// Returns `(base_model_pattern, optional_thinking_level)`.
fn parse_thinking_suffix(pattern: &str) -> (String, Option<ThinkingLevel>) {
    let colon_pos = pattern.rfind(':');
    match colon_pos {
        None => (pattern.to_string(), None),
        Some(pos) => {
            let base = &pattern[..pos];
            let suffix = &pattern[pos + 1..];
            let level = parse_thinking_level(suffix);
            if level.is_some() || suffix.is_empty() {
                (base.to_string(), level)
            } else {
                // The colon is part of the model ID (e.g., "claude-3:opus")
                (pattern.to_string(), None)
            }
        }
    }
}

/// Parse a thinking level string.
fn parse_thinking_level(s: &str) -> Option<ThinkingLevel> {
    match s.to_lowercase().as_str() {
        "off" => Some(ThinkingLevel::Off),
        "minimal" => Some(ThinkingLevel::Minimal),
        "low" => Some(ThinkingLevel::Low),
        "medium" => Some(ThinkingLevel::Medium),
        "high" => Some(ThinkingLevel::High),
        _ => None,
    }
}

// ── Exact Matching ──────────────────────────────────────────────────

/// Exact match by "provider/modelId" form.
///
/// Splits pattern on "/", then matches provider name and model id parts.
fn exact_match_provider_model<'a>(
    pattern: &str,
    available: &'a [&'a ModelMeta],
) -> Option<&'a ModelMeta> {
    // Try "provider/modelId" exact match
    if let Some(slash_pos) = pattern.find('/') {
        let provider = &pattern[..slash_pos];
        let model_id_part = &pattern[slash_pos + 1..];

        // Full id match: e.g., "openai/gpt-4o" matches id "openai/gpt-4o"
        if let Some(found) = available.iter().find(|m| m.id == pattern) {
            return Some(found);
        }

        // Provider + model part match: e.g., "openai/gpt-4o" matches when
        // the model's config.provider_name() == "openai" and id contains "gpt-4o"
        let candidates: Vec<&&ModelMeta> = available
            .iter()
            .filter(|m| m.config.provider_name() == provider)
            .collect();

        if !candidates.is_empty() {
            // Try exact model id part within that provider
            if let Some(found) = candidates.iter().find(|m| {
                let model_cfg = &m.config.model;
                model_cfg == model_id_part || m.id.ends_with(model_id_part)
            }) {
                return Some(found);
            }

            // Try partial model id match
            if let Some(found) = candidates
                .iter()
                .find(|m| m.config.model.contains(model_id_part) || m.id.contains(model_id_part))
            {
                return Some(found);
            }

            // Return first from this provider
            return candidates.first().copied().copied();
        }
    }

    None
}

/// Exact match by bare model id (no provider prefix).
fn exact_match_bare_id<'a>(pattern: &str, available: &'a [&'a ModelMeta]) -> Option<&'a ModelMeta> {
    // Match against config.model (the provider-specific id part)
    let found = available.iter().find(|m| m.config.model == pattern);
    if let Some(m) = found {
        return Some(m);
    }

    // Match against full id
    available.iter().find(|m| m.id == pattern).copied()
}

// ── Fuzzy Matching ──────────────────────────────────────────────────

/// Fuzzy match: substring in id or display_name.
///
/// Prefers shorter ids (aliases without version/date suffixes) over
/// longer dated versions when both match the same pattern.
fn fuzzy_match<'a>(pattern: &str, available: &'a [&'a ModelMeta]) -> Option<&'a ModelMeta> {
    let pattern_lower = pattern.to_lowercase();

    let mut candidates: Vec<&&ModelMeta> = available
        .iter()
        .filter(|m| {
            let id_lower = m.id.to_lowercase();
            let name_lower = m.display_name.to_lowercase();
            id_lower.contains(&pattern_lower) || name_lower.contains(&pattern_lower)
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Sort: prefer shorter ids (aliases without versions/date suffixes)
    // then by exact display name match
    candidates.sort_by_key(|m| {
        let id_len = m.id.len();
        let display_exact = m.display_name.to_lowercase() == pattern_lower;
        (
            !display_exact, // exact match first (false < true)
            id_len,         // shorter ids first (aliases)
        )
    });

    candidates.first().copied().copied()
}

// ── Fallback Model ──────────────────────────────────────────────────

/// Build a fallback model when the requested model ID is not found.
///
/// Strategy:
/// 1. If the pattern looks like "provider/model", use that provider's default model
/// 2. If `default_provider` is given, use its default model
/// 3. Otherwise use the first available model
///
/// The fallback model preserves the user's requested id as a reference name.
pub(crate) fn build_fallback_model(
    pattern: &str,
    available: &[&ModelMeta],
    default_provider: Option<&str>,
) -> Option<ModelMeta> {
    // Try to determine the intended provider from the pattern
    let provider_hint = if let Some(slash_pos) = pattern.find('/') {
        Some(&pattern[..slash_pos])
    } else {
        default_provider
    };

    // Find a model from the hinted provider
    if let Some(provider) = provider_hint
        && let Some(template) = available
            .iter()
            .find(|m| m.config.provider_name() == provider)
    {
        return Some(ModelMeta {
            id: pattern.to_string(),
            config: ModelConfig {
                kind: template.config.kind,
                api_key: template.config.api_key.clone(),
                model: template.config.model.clone(),
                base_url: template.config.base_url.clone(),
            },
            display_name: format!("{} (fallback)", pattern),
            thinking: template.thinking,
            context_window: template.context_window,
            api: String::new(),
            provider: String::new(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
        });
    }

    // Last resort: clone first available with user's requested id
    available.first().map(|template| ModelMeta {
        id: pattern.to_string(),
        config: template.config.clone(),
        display_name: format!("{} (fallback)", pattern),
        thinking: template.thinking,
        context_window: template.context_window,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
    })
}

// ── Model Filtering (enabledModels) ─────────────────────────────────

// ── Helper: apply thinking level to ResolvedModel ───────────────────

impl ResolvedModel {
    fn with_thinking_level_opt(mut self, level: Option<ThinkingLevel>) -> Self {
        self.thinking_level = level;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::ModelConfig;

    fn make_available() -> Vec<ModelMeta> {
        vec![
            ModelMeta {
                id: "openai/gpt-4o".into(),
                config: ModelConfig {
                    kind: ModelKind::OpenAi,
                    api_key: "sk-test".into(),
                    model: "gpt-4o".into(),
                    base_url: None,
                },
                display_name: "GPT-4o".into(),
                thinking: true,
                context_window: 128_000,
                api: String::new(),
                provider: String::new(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: Vec::new(),
            },
            ModelMeta {
                id: "openai/gpt-4o-mini".into(),
                config: ModelConfig {
                    kind: ModelKind::OpenAi,
                    api_key: "sk-test".into(),
                    model: "gpt-4o-mini".into(),
                    base_url: None,
                },
                display_name: "GPT-4o Mini".into(),
                thinking: true,
                context_window: 128_000,
                api: String::new(),
                provider: String::new(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: Vec::new(),
            },
            ModelMeta {
                id: "claude-sonnet-4-20250514".into(),
                config: ModelConfig {
                    kind: ModelKind::Anthropic,
                    api_key: "".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    base_url: None,
                },
                display_name: "Claude Sonnet 4".into(),
                thinking: true,
                context_window: 200_000,
                api: String::new(),
                provider: String::new(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: Vec::new(),
            },
            ModelMeta {
                id: "anthropic/claude-sonnet-4-20250514".into(),
                config: ModelConfig {
                    kind: ModelKind::Anthropic,
                    api_key: "".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    base_url: None,
                },
                display_name: "Claude Sonnet 4".into(),
                thinking: true,
                context_window: 200_000,
                api: String::new(),
                provider: String::new(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: Vec::new(),
            },
        ]
    }

    fn refs(models: &[ModelMeta]) -> Vec<&ModelMeta> {
        models.iter().collect()
    }

    #[test]
    fn test_resolve_exact_provider_model() {
        let models = make_available();
        let available = refs(&models);
        let result = resolve_model("openai/gpt-4o", &available, None).unwrap();
        assert_eq!(result.model.id, "openai/gpt-4o");
        assert!(result.warning.is_none());
    }

    #[test]
    fn test_resolve_exact_bare_id() {
        let models = make_available();
        let available = refs(&models);
        let result = resolve_model("claude-sonnet-4-20250514", &available, None).unwrap();
        assert_eq!(result.model.id, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_resolve_fuzzy_substring() {
        let models = make_available();
        let available = refs(&models);
        let result = resolve_model("4o-mini", &available, None).unwrap();
        assert!(result.model.id.contains("4o-mini"));
        assert!(result.warning.is_some()); // fuzzy match warns
    }

    #[test]
    fn test_resolve_fallback() {
        let models = make_available();
        let available = refs(&models);
        let result = resolve_model("nonexistent-model", &available, None).unwrap();
        assert!(result.warning.is_some());
        assert!(result.warning.unwrap().contains("fallback"));
    }

    #[test]
    fn test_resolve_empty_available() {
        let available: Vec<&ModelMeta> = vec![];
        let result = resolve_model("any", &available, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_thinking_suffix_high() {
        let (base, level) = parse_thinking_suffix("gpt-4o:high");
        assert_eq!(base, "gpt-4o");
        assert_eq!(level, Some(ThinkingLevel::High));
    }

    #[test]
    fn test_parse_thinking_suffix_no_colon() {
        let (base, level) = parse_thinking_suffix("gpt-4o");
        assert_eq!(base, "gpt-4o");
        assert_eq!(level, None);
    }

    #[test]
    fn test_parse_thinking_suffix_not_a_level() {
        // "claude-3:opus" — colon but "opus" is not a thinking level
        let (base, level) = parse_thinking_suffix("claude-3:opus");
        assert_eq!(base, "claude-3:opus");
        assert_eq!(level, None);
    }

    #[test]
    fn test_resolve_with_thinking_level() {
        let models = make_available();
        let available = refs(&models);
        let result = resolve_model("openai/gpt-4o:low", &available, None).unwrap();
        assert_eq!(result.model.id, "openai/gpt-4o");
        assert_eq!(result.thinking_level, Some(ThinkingLevel::Low));
    }

    #[test]
    fn test_resolve_with_default_provider() {
        let models = make_available();
        let available = refs(&models);
        // "gpt-4o-mini" exists as bare match already, but "gpt-4.1" doesn't
        let result = resolve_model("gpt-4.1", &available, Some("openai")).unwrap();
        assert!(result.warning.is_some());
        // Should get a fallback with gpt-4o as the underlying model
        assert_eq!(result.model.id, "gpt-4.1");
        assert_eq!(result.model.config.model, "gpt-4o"); // fallback base from openai
    }

    #[test]
    fn test_build_fallback_model_from_provider() {
        let models = make_available();
        let available = refs(&models);
        let fallback = build_fallback_model("missing-model", &available, Some("openai")).unwrap();
        assert_eq!(fallback.id, "missing-model");
        assert_eq!(fallback.config.kind, ModelKind::OpenAi);
    }

    #[test]
    fn test_alias_preference() {
        let models = vec![
            ModelMeta {
                id: "claude-sonnet".into(),
                config: ModelConfig {
                    kind: ModelKind::Anthropic,
                    api_key: "".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    base_url: None,
                },
                display_name: "Claude Sonnet".into(),
                thinking: true,
                context_window: 200_000,
                api: String::new(),
                provider: String::new(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: Vec::new(),
            },
            ModelMeta {
                id: "claude-sonnet-4-20250514".into(),
                config: ModelConfig {
                    kind: ModelKind::Anthropic,
                    api_key: "".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    base_url: None,
                },
                display_name: "Claude Sonnet 4 (2025-05-14)".into(),
                thinking: true,
                context_window: 200_000,
                api: String::new(),
                provider: String::new(),
                cost_input: 0.0,
                cost_output: 0.0,
                cost_cache_read: 0.0,
                cost_cache_write: 0.0,
                max_tokens: 0,
                thinking_levels: Vec::new(),
            },
        ];
        let available = refs(&models);
        // Both match "claude" — alias (shorter id) should be preferred
        let result = resolve_model("claude", &available, None).unwrap();
        assert_eq!(result.model.id, "claude-sonnet");
    }
}
