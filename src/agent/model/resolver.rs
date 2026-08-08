//! ModelResolver — pattern-based model resolution.
//!
//! Provides:
//! - Exact match: `provider/modelId` or bare `id`
//! - Fuzzy match: partial id or name substring match
//! - Alias preference: shorter/cleaner ids over dated versions
//! - `model:thinkingLevel` suffix parsing
//! - Fallback model construction

use crate::protocol::model::XyModelConfig;
#[cfg(test)]
use crate::protocol::model::XyModelKind;
use crate::protocol::model::XyModelMeta;

// ── Resolved Model ──────────────────────────────────────────────────

/// Result of resolving a model pattern against the available models.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedModel {
    /// The resolved model metadata.
    pub(crate) model: XyModelMeta,
    /// Optional thinking level parsed from `model:level` suffix.
    pub(crate) thinking_level: Option<String>,
    /// Warning message, e.g., when using a fallback.
    pub(crate) warning: Option<String>,
}

impl ResolvedModel {
    pub(crate) fn new(model: XyModelMeta) -> Self {
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
/// 1. Try the complete pattern as an exact model id.
/// 2. For a non-empty `model:thinkingLevel` suffix, resolve the base model and
///    retain the suffix verbatim.
/// 3. Try exact and fuzzy model matching.
/// 4. Fallback to the requested complete pattern.
pub(crate) fn resolve_model(
    pattern: &str,
    available: &[&XyModelMeta],
    default_provider: Option<&str>,
) -> Result<ResolvedModel, String> {
    if available.is_empty() {
        return Err("no models available".to_string());
    }

    // Preserve a colon-bearing configured model id before treating the final
    // segment as a thinking-level suffix.
    if let Some(found) = exact_match_bare_id(pattern, available) {
        return Ok(ResolvedModel::new(found.clone()));
    }

    if let Some((base_pattern, thinking_level)) = parse_thinking_suffix(pattern)
        && let Some(resolved) = resolve_existing_model(base_pattern, available)
    {
        return Ok(resolved.with_thinking_level(thinking_level.to_string()));
    }

    Ok(resolve_model_with_fallback(
        pattern,
        available,
        default_provider,
    ))
}

/// Resolve an already configured model, without synthesizing a fallback.
///
/// This distinction keeps an unknown `model:variant` intact as a model pattern
/// instead of incorrectly treating `variant` as a thinking level.
fn resolve_existing_model(pattern: &str, available: &[&XyModelMeta]) -> Option<ResolvedModel> {
    if let Some(found) = provider_model_match(pattern, available) {
        return Some(ResolvedModel::new(found.clone()));
    }
    if let Some(found) = exact_match_bare_id(pattern, available) {
        return Some(ResolvedModel::new(found.clone()));
    }
    fuzzy_match(pattern, available).map(|found| {
        ResolvedModel::new(found.clone()).with_warning(format!(
            "Fuzzy-matched model '{}' for pattern '{}'",
            found.id, pattern
        ))
    })
}

/// Resolve a pattern while retaining the existing fallback behavior.
fn resolve_model_with_fallback(
    pattern: &str,
    available: &[&XyModelMeta],
    default_provider: Option<&str>,
) -> ResolvedModel {
    // Keep the legacy provider-pattern fallback for a complete model request.
    // Suffix parsing uses `resolve_existing_model` above so this fallback cannot
    // make an unknown base look like a resolved model.
    if let Some(found) = exact_match_provider_model(pattern, available) {
        return ResolvedModel::new(found.clone());
    }
    if let Some(found) = exact_match_bare_id(pattern, available) {
        return ResolvedModel::new(found.clone());
    }
    if let Some(found) = fuzzy_match(pattern, available) {
        return ResolvedModel::new(found.clone()).with_warning(format!(
            "Fuzzy-matched model '{}' for pattern '{}'",
            found.id, pattern
        ));
    }
    if let Some(fallback) = build_fallback_model(pattern, available, default_provider) {
        return ResolvedModel::new(fallback)
            .with_warning(format!("Model '{}' not found. Using fallback.", pattern));
    }

    // Last resort — first available. `available` was checked by resolve_model.
    let first = available.first().expect("non-empty: checked above");
    ResolvedModel::new((*first).clone()).with_warning(format!(
        "Model '{}' not found. Using first available: {}",
        pattern, first.id
    ))
}

// ── Thinking Level Parsing ──────────────────────────────────────────

/// Parse `model:thinkingLevel` suffix.
///
/// Returns a base model pattern and a non-empty freeform suffix.
fn parse_thinking_suffix(pattern: &str) -> Option<(&str, &str)> {
    let (base, suffix) = pattern.rsplit_once(':')?;
    (!base.is_empty() && !suffix.is_empty()).then_some((base, suffix))
}

// ── Exact Matching ──────────────────────────────────────────────────

/// Exact match by "provider/modelId" form.
///
/// Splits pattern on "/", then matches provider name and model id parts.
fn exact_match_provider_model<'a>(
    pattern: &str,
    available: &'a [&'a XyModelMeta],
) -> Option<&'a XyModelMeta> {
    if let Some(found) = provider_model_match(pattern, available) {
        return Some(found);
    }
    let (provider, _) = pattern.split_once('/')?;
    available
        .iter()
        .copied()
        .find(|model| model.config.provider_name() == provider)
}

/// Match a configured provider/model without falling back to another model.
fn provider_model_match<'a>(
    pattern: &str,
    available: &'a [&'a XyModelMeta],
) -> Option<&'a XyModelMeta> {
    let (provider, model_id_part) = pattern.split_once('/')?;
    if let Some(found) = available.iter().copied().find(|model| model.id == pattern) {
        return Some(found);
    }
    let candidates: Vec<&XyModelMeta> = available
        .iter()
        .copied()
        .filter(|model| model.config.provider_name() == provider)
        .collect();
    candidates
        .iter()
        .copied()
        .find(|model| model.config.model == model_id_part || model.id.ends_with(model_id_part))
        .or_else(|| {
            candidates.iter().copied().find(|model| {
                model.config.model.contains(model_id_part) || model.id.contains(model_id_part)
            })
        })
}

/// Exact match by bare model id (no provider prefix).
///
/// Prefer `XyModelMeta.id` (alias / registry key). Only fall back to
/// `config.model` (upstream wire id) when that match is **unique** — otherwise
/// two aliases sharing one upstream id (e.g. `deepseek-v4-flash` vs
/// `deepseek-v4-flash-anthropic`) would pick whichever HashMap/list order wins.
fn exact_match_bare_id<'a>(
    pattern: &str,
    available: &'a [&'a XyModelMeta],
) -> Option<&'a XyModelMeta> {
    if let Some(found) = available.iter().find(|m| m.id == pattern) {
        return Some(*found);
    }

    let by_upstream: Vec<&&XyModelMeta> = available
        .iter()
        .filter(|m| m.config.model == pattern)
        .collect();
    match by_upstream.as_slice() {
        [only] => Some(**only),
        _ => None,
    }
}

// ── Fuzzy Matching ──────────────────────────────────────────────────

/// Fuzzy match: substring in id or display_name.
///
/// Prefers shorter ids (aliases without version/date suffixes) over
/// longer dated versions when both match the same pattern.
fn fuzzy_match<'a>(pattern: &str, available: &'a [&'a XyModelMeta]) -> Option<&'a XyModelMeta> {
    let pattern_lower = pattern.to_lowercase();

    let mut candidates: Vec<&&XyModelMeta> = available
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
    available: &[&XyModelMeta],
    default_provider: Option<&str>,
) -> Option<XyModelMeta> {
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
        return Some(XyModelMeta {
            id: pattern.to_string(),
            config: XyModelConfig {
                kind: template.config.kind,
                api_key: template.config.api_key.clone(),
                model: template.config.model.clone(),
                base_url: template.config.base_url.clone(),
                api: None,
                compat: None,
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
            thinking_level_map: Default::default(),
        });
    }

    // Last resort: clone first available with user's requested id
    available.first().map(|template| XyModelMeta {
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
        thinking_level_map: Default::default(),
    })
}

// ── Helper: apply thinking level to ResolvedModel ───────────────────

impl ResolvedModel {
    fn with_thinking_level(mut self, level: String) -> Self {
        self.thinking_level = Some(level);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::model::XyModelConfig;

    fn make_available() -> Vec<XyModelMeta> {
        vec![
            XyModelMeta {
                id: "openai/gpt-4o".into(),
                config: XyModelConfig {
                    kind: XyModelKind::OpenAi,
                    api_key: "sk-test".into(),
                    model: "gpt-4o".into(),
                    base_url: None,
                    api: None,
                    compat: None,
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
                thinking_level_map: Default::default(),
            },
            XyModelMeta {
                id: "openai/gpt-4o-mini".into(),
                config: XyModelConfig {
                    kind: XyModelKind::OpenAi,
                    api_key: "sk-test".into(),
                    model: "gpt-4o-mini".into(),
                    base_url: None,
                    api: None,
                    compat: None,
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
                thinking_level_map: Default::default(),
            },
            XyModelMeta {
                id: "claude-sonnet-4-20250514".into(),
                config: XyModelConfig {
                    kind: XyModelKind::Anthropic,
                    api_key: "".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    base_url: None,
                    api: None,
                    compat: None,
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
                thinking_level_map: Default::default(),
            },
            XyModelMeta {
                id: "anthropic/claude-sonnet-4-20250514".into(),
                config: XyModelConfig {
                    kind: XyModelKind::Anthropic,
                    api_key: "".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    base_url: None,
                    api: None,
                    compat: None,
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
                thinking_level_map: Default::default(),
            },
        ]
    }

    fn refs(models: &[XyModelMeta]) -> Vec<&XyModelMeta> {
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
    fn test_resolve_bare_id_prefers_alias_over_shared_upstream_model() {
        // Two aliases share upstream wire id `deepseek-v4-flash`; selecting the
        // shorter alias must not land on the anthropic variant via HashMap order.
        let models = vec![
            XyModelMeta {
                id: "deepseek-v4-flash-anthropic".into(),
                config: XyModelConfig {
                    kind: XyModelKind::Anthropic,
                    api_key: "sk".into(),
                    model: "deepseek-v4-flash".into(),
                    base_url: Some("https://api.deepseek.com/anthropic".into()),
                    api: None,
                    compat: None,
                },
                display_name: "DeepSeek V4 Flash (Anthropic)".into(),
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
                thinking_level_map: Default::default(),
            },
            XyModelMeta {
                id: "deepseek-v4-flash".into(),
                config: XyModelConfig {
                    kind: XyModelKind::OpenAi,
                    api_key: "sk".into(),
                    model: "deepseek-v4-flash".into(),
                    base_url: Some("https://api.deepseek.com".into()),
                    api: None,
                    compat: None,
                },
                display_name: "DeepSeek V4 Flash".into(),
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
                thinking_level_map: Default::default(),
            },
        ];
        let available = refs(&models);
        let result = resolve_model("deepseek-v4-flash", &available, None).unwrap();
        assert_eq!(result.model.id, "deepseek-v4-flash");
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
        let available: Vec<&XyModelMeta> = vec![];
        let result = resolve_model("any", &available, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_thinking_suffix_high() {
        assert_eq!(
            parse_thinking_suffix("gpt-4o:high"),
            Some(("gpt-4o", "high"))
        );
    }

    #[test]
    fn test_parse_thinking_suffix_no_colon() {
        assert_eq!(parse_thinking_suffix("gpt-4o"), None);
    }

    #[test]
    fn test_parse_thinking_suffix_is_freeform() {
        assert_eq!(
            parse_thinking_suffix("claude-3:opus"),
            Some(("claude-3", "opus"))
        );
    }

    #[test]
    fn test_resolve_with_thinking_level() {
        let models = make_available();
        let available = refs(&models);
        let result = resolve_model("openai/gpt-4o:vendor-fast", &available, None).unwrap();
        assert_eq!(result.model.id, "openai/gpt-4o");
        assert_eq!(result.thinking_level.as_deref(), Some("vendor-fast"));
    }

    #[test]
    fn test_full_colon_model_id_wins_over_suffix_parsing() {
        let mut models = make_available();
        let mut full_id_model = models[0].clone();
        full_id_model.id = "claude-3:opus".into();
        full_id_model.config.model = "claude-3:opus".into();
        models.push(full_id_model);
        let available = refs(&models);

        let result = resolve_model("claude-3:opus", &available, None).unwrap();
        assert_eq!(result.model.id, "claude-3:opus");
        assert_eq!(result.thinking_level, None);
    }

    #[test]
    fn test_unknown_colon_base_remains_full_fallback_pattern() {
        let models = make_available();
        let available = refs(&models);

        let result = resolve_model("unconfigured:opus", &available, None).unwrap();
        assert_eq!(result.model.id, "unconfigured:opus");
        assert_eq!(result.thinking_level, None);
    }

    #[test]
    fn test_unknown_provider_base_does_not_take_suffix() {
        let models = make_available();
        let available = refs(&models);

        let result = resolve_model("openai/not-configured:opus", &available, None).unwrap();
        assert_eq!(result.thinking_level, None);
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
        assert_eq!(fallback.config.kind, XyModelKind::OpenAi);
    }

    #[test]
    fn test_alias_preference() {
        let models = vec![
            XyModelMeta {
                id: "claude-sonnet".into(),
                config: XyModelConfig {
                    kind: XyModelKind::Anthropic,
                    api_key: "".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    base_url: None,
                    api: None,
                    compat: None,
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
                thinking_level_map: Default::default(),
            },
            XyModelMeta {
                id: "claude-sonnet-4-20250514".into(),
                config: XyModelConfig {
                    kind: XyModelKind::Anthropic,
                    api_key: "".into(),
                    model: "claude-sonnet-4-20250514".into(),
                    base_url: None,
                    api: None,
                    compat: None,
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
                thinking_level_map: Default::default(),
            },
        ];
        let available = refs(&models);
        // Both match "claude" — alias (shorter id) should be preferred
        let result = resolve_model("claude", &available, None).unwrap();
        assert_eq!(result.model.id, "claude-sonnet");
    }
}
