//! ModelRegistry — provider registration, auth checks, and model discovery.
//!
//! Provides:
//! - ProviderConfig registration with priority ordering
//! - API key auth availability checks
//! - Available model listing sorted by provider priority
//! - Default model ID per provider
//! - Header resolution via ConfigValueResolver

use std::collections::HashMap;
use std::sync::Arc;

use crate::protocol::model::XyModelMeta;
use crate::protocol::model::{XyModelConfig, XyModelKind};
use crate::protocol::ports::XySecretResolver;

// ── Provider Config ─────────────────────────────────────────────────

/// Configuration for a model provider.
///
/// Only `openai`-compatible and `anthropic` providers are supported
/// until after 1.0.0.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub priority: u32,
    /// Provider compatibility mode (openai-compatible / anthropic-messages).
    pub api: Option<ProviderApi>,
    /// Additional HTTP headers for this provider.
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Provider API compatibility type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderApi {
    OpenAiCompatible,
    AnthropicMessages,
}

impl ProviderConfig {
    pub fn openai(api_key: Option<String>) -> Self {
        Self {
            name: "openai".to_string(),
            api_key,
            base_url: None,
            priority: 10,
            api: Some(ProviderApi::OpenAiCompatible),
            headers: None,
        }
    }

    pub fn anthropic(api_key: Option<String>) -> Self {
        Self {
            name: "anthropic".to_string(),
            api_key,
            base_url: None,
            priority: 20,
            api: Some(ProviderApi::AnthropicMessages),
            headers: None,
        }
    }

    /// Create a custom provider config (user-defined provider like LM Studio, Ollama).
    /// Custom providers default to OpenAI-compatible API.
    pub fn custom(
        name: &str,
        api: ProviderApi,
        base_url: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            name: name.to_string(),
            api_key,
            base_url: Some(base_url.into()),
            priority: 30,
            api: Some(api),
            headers: None,
        }
    }

    pub fn has_credentials(&self) -> bool {
        self.api_key.is_some()
    }
}

// ── Default Model IDs ───────────────────────────────────────────────

const DEFAULT_MODEL_PER_PROVIDER: &[(&str, &str)] = &[
    ("openai", "gpt-4o"),
    ("anthropic", "claude-sonnet-4-20250514"),
];

pub fn default_model_id_for_provider(provider_name: &str) -> Option<&'static str> {
    DEFAULT_MODEL_PER_PROVIDER
        .iter()
        .find(|(p, _)| *p == provider_name)
        .map(|(_, m)| *m)
}

// ── Model Registry ──────────────────────────────────────────────────

/// Registry of model providers and their available models.
/// This is the canonical `ModelRegistry` used throughout the agent.
#[derive(Clone, Debug)]
pub struct ModelRegistry {
    providers: HashMap<String, ProviderConfig>,
    models: Vec<XyModelMeta>,
    secret_resolver: Arc<dyn XySecretResolver>,
}

impl ModelRegistry {
    pub fn new(secret_resolver: Arc<dyn XySecretResolver>) -> Self {
        Self {
            providers: HashMap::new(),
            models: Vec::new(),
            secret_resolver,
        }
    }

    // ── Provider management ───────────────────────────────────────

    pub fn register_provider(&mut self, name: &str, config: ProviderConfig) {
        self.providers.insert(name.to_string(), config);
    }

    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }

    /// Check if a provider has an API key configured (literal or config-value reference).
    pub fn has_configured_auth(&self, provider_name: &str) -> bool {
        self.providers
            .get(provider_name)
            .is_some_and(|p| p.api_key.is_some())
    }

    /// Check if a provider has a *resolvable* API key — one that can
    /// actually be used at runtime (env var is set, shell command succeeds, etc.).
    pub fn has_resolved_auth(&self, provider_name: &str) -> bool {
        self.providers.get(provider_name).is_some_and(|p| {
            p.api_key
                .as_ref()
                .and_then(|key| self.secret_resolver.resolve_config_value(key, None))
                .is_some()
        })
    }

    /// Resolve custom headers for a provider, interpolating env vars and executing shell commands.
    /// Returns None if the provider has no headers or all headers fail to resolve.
    pub fn resolve_provider_headers(
        &self,
        provider_name: &str,
        env: Option<&HashMap<String, String>>,
    ) -> Option<HashMap<String, String>> {
        let provider = self.providers.get(provider_name)?;
        let headers = provider.headers.as_ref()?;
        self.secret_resolver.resolve_headers(headers, env)
    }

    pub fn has_configured_auth_for_model(&self, model: &XyModelMeta) -> bool {
        let provider_name = model.config.provider_name();
        self.has_configured_auth(provider_name)
    }

    // ── Model management ──────────────────────────────────────────

    pub fn register(&mut self, meta: XyModelMeta) {
        self.models.push(meta);
    }

    pub fn get_available(&self) -> Vec<&XyModelMeta> {
        let mut models: Vec<&XyModelMeta> = self.models.iter().collect();
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

    pub fn default_model_id_for(&self, provider_name: &str) -> Option<&'static str> {
        default_model_id_for_provider(provider_name)
    }

    pub fn find(&self, id: &str) -> Option<&XyModelMeta> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn list(&self) -> &[XyModelMeta] {
        &self.models
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn first_model(&self) -> Option<&XyModelMeta> {
        self.models.first()
    }

    pub fn get_at(&self, index: usize) -> Option<&XyModelMeta> {
        self.models.get(index)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.models.iter().position(|m| m.id == id)
    }

    // ── Diagnostics ───────────────────────────────────────────────

    pub fn auth_guidance_message(&self, model: &XyModelMeta) -> Option<String> {
        let provider_name = model.config.provider_name();
        if self.has_configured_auth(provider_name) {
            return None;
        }

        Some(format!(
            "No API key configured for {provider_name}. \
             Set {env_var} environment variable.",
            env_var = env_var_for_provider(provider_name)
        ))
    }

    pub fn collect_diagnostics(&self) -> Vec<String> {
        let mut diags = Vec::new();

        for (name, provider) in &self.providers {
            if !provider.has_credentials() {
                let env_var = env_var_for_provider(name);
                diags.push(format!(
                    "Warning: No API key configured for {name}. Set {env_var}."
                ));
            }
        }

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

fn env_var_for_provider(name: &str) -> &'static str {
    match name {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "API_KEY",
    }
}

// ── Helpers ────────────────────────────────────────────────────────

pub fn build_default_model_meta(provider: &ProviderConfig) -> Option<XyModelMeta> {
    let model_id = default_model_id_for_provider(&provider.name)?;
    let kind = match provider.name.as_str() {
        "openai" => XyModelKind::OpenAi,
        "anthropic" => XyModelKind::Anthropic,
        "fake" => XyModelKind::Fake,
        _ => return None,
    };

    Some(XyModelMeta {
        id: format!("{}/{}", provider.name, model_id),
        config: XyModelConfig {
            kind,
            api_key: provider.api_key.clone()?,
            model: model_id.to_string(),
            base_url: provider.base_url.clone(),
            api: None,
            compat: None,
        },
        display_name: format!("{} ({})", model_id, provider.name),
        thinking: matches!(kind, XyModelKind::OpenAi | XyModelKind::Anthropic),
        context_window: default_context_window_for(kind),
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

pub use crate::protocol::model::default_context_window_for;

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_registry() -> ModelRegistry {
        ModelRegistry::new(Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ))
    }

    fn make_test_registry() -> ModelRegistry {
        let mut reg = empty_registry();
        reg.register_provider("openai", ProviderConfig::openai(Some("sk-test".into())));
        reg.register_provider(
            "anthropic",
            ProviderConfig {
                name: "anthropic".into(),
                api_key: None,
                base_url: None,
                priority: 20,
                api: Some(ProviderApi::AnthropicMessages),
                headers: None,
            },
        );
        reg.register(XyModelMeta {
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
        });
        reg.register(XyModelMeta {
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
        });
        reg.register(XyModelMeta {
            id: "claude-sonnet-4-20250514".into(),
            config: XyModelConfig {
                kind: XyModelKind::Anthropic,
                api_key: String::new(),
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
        });
        reg
    }

    #[test]
    fn test_register_provider() {
        let mut reg = empty_registry();
        reg.register_provider("openai", ProviderConfig::openai(Some("sk-key".into())));
        assert!(reg.has_provider("openai"));
        assert!(reg.has_configured_auth("openai"));
    }

    #[test]
    fn test_has_configured_auth_false() {
        let mut reg = empty_registry();
        reg.register_provider("anthropic", ProviderConfig::anthropic(None));
        assert!(!reg.has_configured_auth("anthropic"));
    }

    #[test]
    fn test_has_configured_auth_config_value_present() {
        let mut reg = empty_registry();
        reg.register_provider(
            "openai",
            ProviderConfig::openai(Some("$OPENAI_API_KEY".into())),
        );
        // Config value is present (even though env var may not be set)
        assert!(reg.has_configured_auth("openai"));
    }

    #[test]
    fn test_has_resolved_auth_checks_env_var() {
        let mut reg = empty_registry();
        reg.register_provider(
            "openai",
            ProviderConfig::openai(Some("$THIS_ENV_VAR_SHOULD_NOT_EXIST_XYZ".into())),
        );
        // Without the env var set, should be false
        assert!(!reg.has_resolved_auth("openai"));
    }

    #[test]
    fn test_get_available_sorted_by_priority() {
        let reg = make_test_registry();
        let available = reg.get_available();
        assert_eq!(available.len(), 3);
        assert!(available[0].id.starts_with("openai"));
        assert!(available[1].id.starts_with("openai"));
        assert!(available[2].id.starts_with("claude"));
    }

    #[test]
    fn test_default_model_id() {
        let openai = default_model_id_for_provider("openai").expect("openai default");
        let anthropic = default_model_id_for_provider("anthropic").expect("anthropic default");
        // Defaults MUST be real, currently-shipping model identifiers — never
        // placeholders or non-existent model names (regression guard for the
        // gpt-5.4 / claude-opus-4-8 placeholder bug).
        assert_eq!(openai, "gpt-4o");
        assert_eq!(anthropic, "claude-sonnet-4-20250514");
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
        assert!(diags.iter().any(|d| d.contains("anthropic")));
    }
}
