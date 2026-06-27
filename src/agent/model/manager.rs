//! ModelManager — model registry, selection, and thinking level.
//!
//! Extracted from [`AgentSession`](super::session::AgentSession) to isolate
//! model-related responsibilities into a focused component.

use std::sync::Arc;

use crate::agent::model::registry::ModelRegistry;
use crate::core::model::ModelConfig;
use crate::core::ports::XyModel;
use crate::core::types::{ModelMeta, ThinkingLevel};

/// Manages model registry, current model selection, and thinking level.
///
/// Owned by [`AgentSession`](super::session::AgentSession) as a composed field.
/// The provider is built via an injected `model_builder` (HC-1: agent must not
/// call `infra::provider::factory::build_provider` directly; the composition
/// root supplies the builder).
pub struct ModelManager {
    /// Model registry for provider/model lookup.
    pub(crate) registry: ModelRegistry,
    /// Index of the currently selected model in the registry.
    pub(crate) current_index: usize,
    /// Current thinking level (clamped to model capabilities).
    pub(crate) thinking_level: ThinkingLevel,
    /// Injected provider factory (composition-root-supplied).
    pub(crate) model_builder:
        Arc<dyn Fn(&ModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>,
}

impl ModelManager {
    /// Create a new ModelManager with the given registry, default index, and
    /// injected provider builder.
    pub fn new(
        registry: ModelRegistry,
        model_builder: Arc<dyn Fn(&ModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>,
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
    pub fn current_model(&self) -> Option<&ModelMeta> {
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

    /// Cycle to the next model in the registry.
    pub fn cycle_forward(&mut self) -> Option<&ModelMeta> {
        let len = self.registry.len();
        if len == 0 {
            return None;
        }
        let next = (self.current_index + 1) % len;
        self.current_index = next;
        self.registry.list().get(next)
    }

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

    /// Clone the underlying model registry.
    pub fn registry_clone(&self) -> ModelRegistry {
        self.registry.clone()
    }

    /// Get the index of the currently selected model.
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Get the ModelConfig for the current model.
    pub fn current_config(&self) -> Option<ModelConfig> {
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
    use crate::core::model::ModelConfig;
    use crate::core::ports::XyModel;

    fn empty_registry() -> ModelRegistry {
        ModelRegistry::new(Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ))
    }

    /// A fake model builder that always reports "no model configured";
    /// tests don't build real providers.
    fn fake_builder() -> Arc<dyn Fn(&ModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>
    {
        Arc::new(|_cfg: &ModelConfig| Err("test: no provider".to_string()))
    }

    #[test]
    fn new_model_manager_empty_registry() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert!(mm.current_model().is_none());
        assert_eq!(mm.current_index(), 0);
        // No model means no thinking support → clamped to Off
        assert_eq!(mm.thinking_level(), crate::core::types::ThinkingLevel::Off);
    }

    #[test]
    fn new_model_manager_default_thinking() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert_eq!(mm.thinking_level, crate::core::types::ThinkingLevel::Medium);
    }

    #[test]
    fn set_thinking_level() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        mm.set_thinking_level(crate::core::types::ThinkingLevel::Low);
        assert_eq!(mm.thinking_level, crate::core::types::ThinkingLevel::Low);
    }

    #[test]
    fn set_thinking_level_high() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        mm.set_thinking_level(crate::core::types::ThinkingLevel::High);
        assert_eq!(mm.thinking_level, crate::core::types::ThinkingLevel::High);
    }

    #[test]
    fn cycle_forward_empty_registry_returns_none() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        assert!(mm.cycle_forward().is_none());
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
    fn registry_clone() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        let cloned = mm.registry_clone();
        assert_eq!(cloned.list().len(), 0);
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
