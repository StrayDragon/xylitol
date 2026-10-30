use std::collections::HashMap;

use xylitol_plugin_sdk::model::{XyModality, XyModelMeta, default_input_modalities};

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub priority: u32,
    pub api: Option<ProviderApi>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

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

#[derive(Clone, Debug)]
pub struct ModelRegistry {
    providers: HashMap<String, ProviderConfig>,
    models: Vec<XyModelMeta>,
    modalities: HashMap<String, Vec<XyModality>>,
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
            modalities: HashMap::new(),
        }
    }

    pub fn register_provider(&mut self, name: &str, config: ProviderConfig) {
        self.providers.insert(name.to_string(), config);
    }

    pub fn register(&mut self, meta: XyModelMeta) {
        let modalities = default_input_modalities(meta.config.kind);
        self.register_with_modalities(meta, modalities);
    }

    pub fn register_with_modalities(&mut self, meta: XyModelMeta, modalities: Vec<XyModality>) {
        self.modalities.insert(meta.id.clone(), modalities);
        self.models.push(meta);
    }

    #[must_use]
    pub fn supports_modality(&self, id: &str, modality: XyModality) -> bool {
        self.modalities
            .get(id)
            .map(|modalities| modalities.contains(&modality))
            .or_else(|| {
                self.find(id)
                    .map(|meta| default_input_modalities(meta.config.kind).contains(&modality))
            })
            .unwrap_or(false)
    }

    #[must_use]
    pub fn modalities_for(&self, id: &str) -> Option<&[XyModality]> {
        self.modalities.get(id).map(Vec::as_slice)
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

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind};

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

    #[test]
    fn supports_modality_uses_provider_default() {
        let reg = make_test_registry();
        assert!(reg.supports_modality("openai/gpt-4o", XyModality::Image));
        assert!(!reg.supports_modality("openai/gpt-4o", XyModality::Audio));
        assert!(!reg.supports_modality("missing", XyModality::Text));
    }

    #[test]
    fn register_with_modalities_overrides_provider_default() {
        let mut reg = empty_registry();
        let meta = make_test_registry()
            .find("openai/gpt-4o")
            .expect("fixture model")
            .clone();
        reg.register_with_modalities(meta, vec![XyModality::Text]);
        assert!(reg.supports_modality("openai/gpt-4o", XyModality::Text));
        assert!(!reg.supports_modality("openai/gpt-4o", XyModality::Image));
    }
}
