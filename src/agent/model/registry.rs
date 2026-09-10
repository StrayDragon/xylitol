//! ModelRegistry — provider registration and model discovery.
//!
//! Provides:
//! - ProviderConfig registration with priority ordering
//! - Available model listing sorted by provider priority
//! - Default model ID per provider

use std::collections::HashMap;

use crate::protocol::model::XyModelMeta;

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

// ── Model Registry ──────────────────────────────────────────────────

/// Registry of model providers and their available models.
/// This is the canonical `ModelRegistry` used throughout the agent.
#[derive(Clone, Debug)]
pub struct ModelRegistry {
    providers: HashMap<String, ProviderConfig>,
    models: Vec<XyModelMeta>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            models: Vec::new(),
        }
    }

    // ── Provider management ───────────────────────────────────────

    pub fn register_provider(&mut self, name: &str, config: ProviderConfig) {
        self.providers.insert(name.to_string(), config);
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
}

pub use crate::protocol::model::default_context_window_for;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::model::{XyModelConfig, XyModelKind};

    fn empty_registry() -> ModelRegistry {
        ModelRegistry::new()
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
    fn test_get_available_sorted_by_priority() {
        let reg = make_test_registry();
        let available = reg.get_available();
        assert_eq!(available.len(), 3);
        assert!(available[0].id.starts_with("openai"));
        assert!(available[1].id.starts_with("openai"));
        assert!(available[2].id.starts_with("claude"));
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
}
