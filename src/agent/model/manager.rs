//! ModelManager — model registry, selection, and thinking level.
//!
//! Extracted from [`AgentCapabilities`](crate::agent::session::AgentCapabilities) to isolate
//! model-related responsibilities into a focused component.

use std::sync::Arc;

use crate::agent::model::registry::ModelRegistry;
use crate::domain::model::XyModelConfig;
use crate::domain::types::{ThinkingLevel, XyModelMeta};
use crate::runtime_protocol::XyModel;

/// Manages model registry, current model selection, and thinking level.
///
/// Owned by [`AgentCapabilities`](crate::agent::session::AgentCapabilities) as a composed field.
/// The provider is built via an injected `model_builder` (the agent layer must
/// not call `infra::provider::factory::build_provider` directly; the composition
/// root supplies the builder).
pub struct ModelManager {
    /// Model registry for provider/model lookup.
    pub(crate) registry: ModelRegistry,
    /// Index of the currently selected model in the registry.
    pub(crate) current_index: usize,
    /// Current thinking level (clamped to model capabilities).
    pub(crate) thinking_level: ThinkingLevel,
    /// Injected provider factory (composition-root-supplied).
    pub(crate) model_builder: crate::runtime_protocol::XyModelBuilder,
}

impl ModelManager {
    /// Create a new ModelManager with the given registry, default index, and
    /// injected provider builder.
    pub fn new(
        registry: ModelRegistry,
        model_builder: crate::runtime_protocol::XyModelBuilder,
    ) -> Self {
        Self {
            registry,
            current_index: 0,
            thinking_level: ThinkingLevel::default(),
            model_builder,
        }
    }

    // ── Current model ────────────────────────────────────────────

    /// Get the currently selected model metadata.
    pub fn current_model(&self) -> Option<&XyModelMeta> {
        self.registry.list().get(self.current_index)
    }

    /// Build a provider instance from the current model config (via the
    /// injected builder).
    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, String> {
        let meta = self
            .current_model()
            .ok_or_else(|| "no model configured".to_string())?;
        (self.model_builder)(&meta.config)
    }

    // ── Thinking level ───────────────────────────────────────────

    /// Get the current thinking level (already clamped).
    pub fn thinking_level(&self) -> ThinkingLevel {
        let supports = self.current_model().map(|m| m.thinking).unwrap_or(false);
        self.thinking_level.clamp(supports)
    }

    /// Set a new thinking level (will be clamped by model capability).
    pub fn set_thinking_level(&mut self, level: ThinkingLevel) {
        self.thinking_level = level;
    }

    // ── Model switching ──────────────────────────────────────────

    /// Select a model by its ID.
    pub fn select_model(&mut self, model_id: &str) -> Result<(), String> {
        let model = self
            .registry
            .find(model_id)
            .ok_or_else(|| format!("model not found: {model_id}"))?;
        // Find index by identity
        let idx = self
            .registry
            .list()
            .iter()
            .position(|m| std::ptr::eq(m, model))
            .unwrap_or(0);
        self.current_index = idx;
        Ok(())
    }

    // ── Accessors ────────────────────────────────────────────────

    /// Get the model registry (read-only).
    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }

    /// Get the index of the currently selected model.
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Get the XyModelConfig for the current model.
    pub fn current_config(&self) -> Option<XyModelConfig> {
        self.registry
            .list()
            .get(self.current_index)
            .map(|m| m.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::ModelManager;
    use crate::agent::model::registry::ModelRegistry;
    use crate::domain::model::XyModelConfig;
    use crate::runtime_protocol::XyModel;

    type ModelBuilderFn =
        Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>;

    fn empty_registry() -> ModelRegistry {
        ModelRegistry::new(Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ))
    }

    /// A fake model builder that always reports "no model configured";
    /// tests don't build real providers.
    fn fake_builder() -> ModelBuilderFn {
        Arc::new(|_cfg: &XyModelConfig| Err("test: no provider".to_string()))
    }

    #[test]
    fn new_model_manager_empty_registry() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert!(mm.current_model().is_none());
        assert_eq!(mm.current_index(), 0);
        // No model means no thinking support → clamped to Off
        assert_eq!(
            mm.thinking_level(),
            crate::domain::types::ThinkingLevel::Off
        );
    }

    #[test]
    fn new_model_manager_default_thinking() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert_eq!(
            mm.thinking_level,
            crate::domain::types::ThinkingLevel::Medium
        );
    }

    #[test]
    fn set_thinking_level() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        mm.set_thinking_level(crate::domain::types::ThinkingLevel::Low);
        assert_eq!(mm.thinking_level, crate::domain::types::ThinkingLevel::Low);
    }

    #[test]
    fn set_thinking_level_high() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        mm.set_thinking_level(crate::domain::types::ThinkingLevel::High);
        assert_eq!(mm.thinking_level, crate::domain::types::ThinkingLevel::High);
    }

    #[test]
    fn select_model_empty_registry_returns_error() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        let result = mm.select_model("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("model not found"));
    }

    #[test]
    fn registry_accessor() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert_eq!(mm.registry().list().len(), 0);
    }

    #[test]
    fn current_config_empty_returns_none() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert!(mm.current_config().is_none());
    }

    #[test]
    fn build_current_model_empty_registry_returns_error() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        let result = mm.build_current_model();
        assert_eq!(result.err().as_deref(), Some("no model configured"));
    }
}
