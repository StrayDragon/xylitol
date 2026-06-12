//! ModelRegistry — provider registration, auth checks, and model discovery.
//!
//! Aligns with pi's model-registry.ts. Provides:
//! - ProviderConfig registration with priority ordering
//! - API key / OAuth auth availability checks
//! - Available model listing sorted by provider priority
//! - Default model ID per provider

#![allow(dead_code)]
use std::collections::HashMap;

use crate::agent::model::{ModelConfig, ModelKind};
use crate::agent::session::ModelMeta;

// ── Provider Config ─────────────────────────────────────────────────

/// Configuration for a model provider.
///
/// Providers are registered with a name, optional API key, base URL override,
/// priority for ordering, and OAuth flag.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider name (e.g., "openai", "anthropic").
    pub(crate) name: String,
    /// API key if configured via env or config file.
    pub(crate) api_key: Option<String>,
    /// Custom base URL for self-hosted / compatible endpoints.
    pub(crate) base_url: Option<String>,
    /// Sort priority (lower = higher priority). Default 100.
    pub(crate) priority: u32,
    /// Whether this provider uses OAuth instead of API key.
    pub(crate) is_oauth: bool,
}

#[allow(dead_code)]
impl ProviderConfig {
    /// Create a new provider config with OpenAI defaults.
    pub(crate) fn openai(api_key: Option<String>) -> Self {
        Self {
            name: "openai".to_string(),
            api_key,
            base_url: None,
            priority: 10,
            is_oauth: false,
        }
    }

    /// Create a new provider config with Anthropic defaults.
    pub(crate) fn anthropic(api_key: Option<String>) -> Self {
        Self {
            name: "anthropic".to_string(),
            api_key,
            base_url: None,
            priority: 20,
            is_oauth: false,
        }
    }

    /// Check if this provider has configured credentials.
    pub(crate) fn has_credentials(&self) -> bool {
        self.is_oauth || self.api_key.is_some()
    }
}

// ── Default Model IDs ───────────────────────────────────────────────

/// Default model ID for each known provider.
///
/// When a user does not specify a model, the default for their configured
/// provider is used. These defaults are the latest stable models as of
/// the reference date.
const DEFAULT_MODEL_PER_PROVIDER: &[(&str, &str)] = &[
    ("openai", "gpt-4o"),
    ("anthropic", "claude-sonnet-4-20250514"),
];

/// Get the default model ID for a provider.
pub fn default_model_id_for_provider(provider_name: &str) -> Option<&'static str> {
    DEFAULT_MODEL_PER_PROVIDER
        .iter()
        .find(|(p, _)| *p == provider_name)
        .map(|(_, m)| *m)
}

// ── Model Registry ──────────────────────────────────────────────────

/// Registry of model providers and their available models.
///
/// Manages provider configuration, auth checks, and model discovery.
#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub(crate) struct ModelRegistry {
    /// Registered providers by name.
    providers: HashMap<String, ProviderConfig>,
    /// All registered models.
    models: Vec<ModelMeta>,
}

impl ModelRegistry {
    /// Create an empty registry.
    pub(crate) fn new() -> Self {
        Self {
            providers: HashMap::new(),
            models: Vec::new(),
        }
    }

    // ── Provider management ───────────────────────────────────────

    /// Register a provider configuration.
    pub(crate) fn register_provider(&mut self, name: &str, config: ProviderConfig) {
        self.providers.insert(name.to_string(), config);
    }

    /// Check if a provider is registered.
    pub(crate) fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    /// Get a provider config by name.
    pub(crate) fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }

    /// Check if a provider has configured authentication.
    ///
    /// Returns `true` if the provider has an API key or is configured for OAuth.
    pub(crate) fn has_configured_auth(&self, provider_name: &str) -> bool {
        self.providers
            .get(provider_name)
            .map(|p| p.has_credentials())
            .unwrap_or(false)
    }

    /// Check if a specific model has available authentication.
    pub(crate) fn has_configured_auth_for_model(&self, model: &ModelMeta) -> bool {
        let provider_name = model.config.provider_name();
        self.has_configured_auth(provider_name)
    }

    // ── Model management ──────────────────────────────────────────

    /// Register a model.
    pub(crate) fn register(&mut self, meta: ModelMeta) {
        self.models.push(meta);
    }

    /// Get all available models, sorted by provider priority.
    ///
    /// Models from higher-priority providers (lower priority number)
    /// appear first. Unknown providers are sorted after all known ones.
    pub(crate) fn get_available(&self) -> Vec<&ModelMeta> {
        let mut models: Vec<&ModelMeta> = self.models.iter().collect();
        models.sort_by(|a, b| {
            let pa = self
                .providers
                .get(a.config.provider_name())
                .map(|p| p.priority)
                .unwrap_or(u32::MAX);
            let pb = self
                .providers
                .get(b.config.provider_name())
                .map(|p| p.priority)
                .unwrap_or(u32::MAX);
            pa.cmp(&pb)
        });
        models
    }

    /// Get the default model ID for a given provider.
    pub(crate) fn default_model_id(&self, provider_name: &str) -> Option<&'static str> {
        default_model_id_for_provider(provider_name)
    }

    /// Find a model by exact ID match.
    pub(crate) fn find(&self, id: &str) -> Option<&ModelMeta> {
        self.models.iter().find(|m| m.id == id)
    }

    /// List all registered models (in insertion order).
    pub(crate) fn list(&self) -> &[ModelMeta] {
        &self.models
    }

    /// Number of registered models.
    pub(crate) fn len(&self) -> usize {
        self.models.len()
    }

    /// Check if the registry is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Get the first model (for fallback when nothing else matches).
    pub(crate) fn first_model(&self) -> Option<&ModelMeta> {
        self.models.first()
    }

    // ── Diagnostics ───────────────────────────────────────────────

    /// Build an auth-guidance error when no auth is configured for a model.
    ///
    /// Returns a user-facing message guiding to use `/login`.
    pub(crate) fn auth_guidance_message(&self, model: &ModelMeta) -> Option<String> {
        let provider_name = model.config.provider_name();
        if self.has_configured_auth(provider_name) {
            return None;
        }

        Some(format!(
            "No API key configured for {provider_name}. \
             Set {env_var} environment variable or use /login to configure.",
            env_var = env_var_for_provider(provider_name)
        ))
    }

    /// Collect diagnostics about provider configurations.
    pub(crate) fn collect_diagnostics(&self) -> Vec<String> {
        let mut diags = Vec::new();

        for (name, provider) in &self.providers {
            if !provider.has_credentials() {
                let env_var = env_var_for_provider(name);
                diags.push(format!(
                    "Warning: No API key configured for {name}. Set {env_var} or use /login."
                ));
            }
        }

        // Check for models with unknown providers
        for model in &self.models {
            let pname = model.config.provider_name();
            if !self.providers.contains_key(pname) {
                diags.push(format!(
                    "Warning: Model '{}' has unregistered provider '{}'",
                    model.id, pname
                ));
            }
        }

        diags
    }
}

/// Get environment variable name for a provider.
fn env_var_for_provider(name: &str) -> &'static str {
    match name {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "API_KEY",
    }
}

// ── Helpers for building ModelMeta from provider config ─────────────

/// Create a default ModelMeta for the given provider using its default model.
pub fn build_default_model_meta(provider: &ProviderConfig) -> Option<ModelMeta> {
    let model_id = default_model_id_for_provider(&provider.name)?;
    let kind = match provider.name.as_str() {
        "openai" => ModelKind::OpenAi,
        "anthropic" => ModelKind::Anthropic,
        #[cfg(feature = "dev-fake-provider")]
        "fake" => ModelKind::Fake,
        _ => return None,
    };

    Some(ModelMeta {
        id: format!("{}/{}", provider.name, model_id),
        config: ModelConfig {
            kind,
            api_key: provider.api_key.clone()?,
            model: model_id.to_string(),
            base_url: provider.base_url.clone(),
        },
        display_name: format!("{} ({})", model_id, provider.name),
        thinking: matches!(kind, ModelKind::OpenAi | ModelKind::Anthropic),
        context_window: default_context_window_for(kind),
    })
}

/// Default context window sizes.
pub fn default_context_window_for(kind: ModelKind) -> u64 {
    match kind {
        ModelKind::OpenAi => 128_000,
        ModelKind::Anthropic => 200_000,
        #[cfg(feature = "dev-fake-provider")]
        ModelKind::Fake => 8_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_registry() -> ModelRegistry {
        let mut reg = ModelRegistry::new();
        reg.register_provider("openai", ProviderConfig::openai(Some("sk-test".into())));
        reg.register_provider(
            "anthropic",
            ProviderConfig {
                name: "anthropic".into(),
                api_key: None,
                base_url: None,
                priority: 20,
                is_oauth: false,
            },
        );
        reg.register(ModelMeta {
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
        });
        reg.register(ModelMeta {
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
        });
        reg.register(ModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            config: ModelConfig {
                kind: ModelKind::Anthropic,
                api_key: String::new(),
                model: "claude-sonnet-4-20250514".into(),
                base_url: None,
            },
            display_name: "Claude Sonnet 4".into(),
            thinking: true,
            context_window: 200_000,
        });
        reg
    }

    #[test]
    fn test_register_provider() {
        let mut reg = ModelRegistry::new();
        reg.register_provider("openai", ProviderConfig::openai(Some("sk-key".into())));
        assert!(reg.has_provider("openai"));
        assert!(reg.has_configured_auth("openai"));
    }

    #[test]
    fn test_has_configured_auth_false() {
        let mut reg = ModelRegistry::new();
        reg.register_provider("anthropic", ProviderConfig::anthropic(None));
        assert!(!reg.has_configured_auth("anthropic"));
    }

    #[test]
    fn test_has_configured_auth_oauth() {
        let mut reg = ModelRegistry::new();
        reg.register_provider(
            "openai",
            ProviderConfig {
                name: "openai".into(),
                api_key: None,
                base_url: None,
                priority: 10,
                is_oauth: true,
            },
        );
        assert!(reg.has_configured_auth("openai"));
    }

    #[test]
    fn test_get_available_sorted_by_priority() {
        let reg = make_test_registry();
        let available = reg.get_available();
        assert_eq!(available.len(), 3);
        // openai (priority 10) should come before anthropic (priority 20)
        assert!(available[0].id.starts_with("openai"));
        assert!(available[1].id.starts_with("openai"));
        assert!(available[2].id.starts_with("claude"));
    }

    #[test]
    fn test_default_model_id() {
        assert_eq!(default_model_id_for_provider("openai"), Some("gpt-4o"));
        assert_eq!(
            default_model_id_for_provider("anthropic"),
            Some("claude-sonnet-4-20250514")
        );
        assert_eq!(default_model_id_for_provider("unknown"), None);
    }

    #[test]
    fn test_find_exact_match() {
        let reg = make_test_registry();
        let found = reg.find("openai/gpt-4o");
        assert!(found.is_some());
        assert_eq!(found.unwrap().display_name, "GPT-4o");
    }

    #[test]
    fn test_find_no_match() {
        let reg = make_test_registry();
        assert!(reg.find("nonexistent").is_none());
    }

    #[test]
    fn test_auth_guidance_message() {
        let reg = make_test_registry();
        let model = reg.find("claude-sonnet-4-20250514").unwrap();
        let msg = reg.auth_guidance_message(model);
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_auth_guidance_message_when_key_exists() {
        let reg = make_test_registry();
        let model = reg.find("openai/gpt-4o").unwrap();
        let msg = reg.auth_guidance_message(model);
        assert!(msg.is_none());
    }

    #[test]
    fn test_collect_diagnostics() {
        let reg = make_test_registry();
        let diags = reg.collect_diagnostics();
        // Should warn about missing anthropic key
        assert!(diags.iter().any(|d| d.contains("anthropic")));
    }
}
