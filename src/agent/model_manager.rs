//! ModelManager — model registry, selection, and thinking level.
//!
//! Extracted from [`AgentSession`](super::session::AgentSession) to isolate
//! model-related responsibilities into a focused component.

use std::sync::Arc;

use crate::agent::model::config::ModelConfigExt;
use crate::agent::model::registry::ModelRegistry;
use crate::core::model::ModelConfig;
use crate::core::traits::XyModel;
use crate::core::types::{ModelMeta, ThinkingLevel};

/// Manages model registry, current model selection, and thinking level.
///
/// Owned by [`AgentSession`](super::session::AgentSession) as a composed field.
#[derive(Debug)]
pub struct ModelManager {
    /// Model registry for provider/model lookup.
    pub(crate) registry: ModelRegistry,
    /// Index of the currently selected model in the registry.
    pub(crate) current_index: usize,
    /// Current thinking level (clamped to model capabilities).
    pub(crate) thinking_level: ThinkingLevel,
}

impl ModelManager {
    /// Create a new ModelManager with the given registry and default index.
    pub fn new(registry: ModelRegistry) -> Self {
        Self {
            registry,
            current_index: 0,
            thinking_level: ThinkingLevel::default(),
        }
    }

    // ── Current model ────────────────────────────────────────────

    /// Get the currently selected model metadata.
    pub fn current_model(&self) -> Option<&ModelMeta> {
        self.registry.list().get(self.current_index)
    }

    /// Build a provider instance from the current model config.
    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, String> {
        let meta = self
            .current_model()
            .ok_or_else(|| "no model configured".to_string())?;
        meta.config.build()
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
