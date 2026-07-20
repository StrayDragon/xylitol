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
    /// Index of the currently selected model in the registry (`None` = unset).
    pub(crate) current_index: Option<usize>,
    /// Current thinking level (clamped to model capabilities).
    pub(crate) thinking_level: ThinkingLevel,
    /// Preferred default from Settings (`default_thinking_level`), if any.
    preferred_default: Option<ThinkingLevel>,
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
            current_index: None,
            thinking_level: ThinkingLevel::default(),
            preferred_default: None,
            model_builder,
        }
    }

    /// Store Settings `default_thinking_level` for clamp/startup.
    pub fn set_preferred_default(&mut self, level: Option<ThinkingLevel>) {
        self.preferred_default = level;
    }

    // ── Current model ────────────────────────────────────────────

    /// Get the currently selected model metadata.
    pub fn current_model(&self) -> Option<&XyModelMeta> {
        let idx = self.current_index?;
        self.registry.list().get(idx)
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

    fn supported_levels(&self) -> Option<Vec<ThinkingLevel>> {
        let meta = self.current_model()?;
        Some(Self::levels_for_meta(meta))
    }

    fn levels_for_meta(meta: &XyModelMeta) -> Vec<ThinkingLevel> {
        if !meta.thinking {
            return vec![ThinkingLevel::Off];
        }
        if meta.thinking_levels.is_empty() {
            return ThinkingLevel::STANDARD.to_vec();
        }
        meta.thinking_levels
            .iter()
            .filter_map(|s| ThinkingLevel::parse(s))
            .collect()
    }

    /// Get the current thinking level (already clamped to support / bool).
    pub fn thinking_level(&self) -> ThinkingLevel {
        match self.supported_levels() {
            Some(levels) => ThinkingLevel::clamp_to_supported(
                self.thinking_level,
                &levels,
                self.preferred_default,
            ),
            None => {
                let supports = false;
                self.thinking_level.clamp(supports)
            }
        }
    }

    /// Set a new thinking level. Rejects if current model has a support set
    /// that does not include `level` (current value unchanged).
    pub fn set_thinking_level(&mut self, level: ThinkingLevel) -> Result<(), String> {
        if let Some(levels) = self.supported_levels()
            && !levels.contains(&level)
        {
            return Err(format!(
                "thinking level `{}` is not supported by the current model",
                level.as_str()
            ));
        }
        self.thinking_level = level;
        Ok(())
    }

    /// After model switch / startup: keep current if still legal, else clamp.
    pub fn clamp_thinking_to_model(&mut self) {
        let Some(levels) = self.supported_levels() else {
            self.thinking_level = ThinkingLevel::Off;
            return;
        };
        self.thinking_level =
            ThinkingLevel::clamp_to_supported(self.thinking_level, &levels, self.preferred_default);
    }

    /// Cycle to the next level in the current model's support list.
    pub fn cycle_thinking_level(&mut self) -> Result<ThinkingLevel, String> {
        let levels = self
            .supported_levels()
            .ok_or_else(|| "no model configured".to_string())?;
        if levels.is_empty() {
            return Err("current model has no thinking levels".into());
        }
        let cur = self.thinking_level();
        let idx = levels.iter().position(|l| *l == cur).unwrap_or(0);
        let next = levels[(idx + 1) % levels.len()];
        self.thinking_level = next;
        Ok(next)
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
        self.current_index = Some(idx);
        self.clamp_thinking_to_model();
        Ok(())
    }

    // ── Accessors ────────────────────────────────────────────────

    /// Get the model registry (read-only).
    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }

    /// Get the index of the currently selected model (`None` if unset).
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    /// Get the XyModelConfig for the current model.
    pub fn current_config(&self) -> Option<XyModelConfig> {
        let idx = self.current_index?;
        self.registry.list().get(idx).map(|m| m.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::ModelManager;
    use crate::agent::model::registry::ModelRegistry;
    use crate::domain::model::{XyModelConfig, XyModelKind};
    use crate::domain::types::{ThinkingLevel, XyModelMeta};
    use crate::runtime_protocol::XyModel;

    type ModelBuilderFn =
        Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>;

    fn empty_registry() -> ModelRegistry {
        ModelRegistry::new(Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ))
    }

    fn fake_builder() -> ModelBuilderFn {
        Arc::new(|_cfg: &XyModelConfig| Err("test: no provider".to_string()))
    }

    fn meta(id: &str, thinking: bool, levels: &[&str]) -> XyModelMeta {
        XyModelMeta {
            id: id.into(),
            config: XyModelConfig {
                kind: XyModelKind::Fake,
                api_key: String::new(),
                model: id.into(),
                base_url: None,
                api: None,
            },
            display_name: id.into(),
            thinking,
            context_window: 0,
            api: String::new(),
            provider: "fake".into(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: levels.iter().map(|s| (*s).to_string()).collect(),
            thinking_level_map: Default::default(),
        }
    }

    fn manager_with(models: Vec<XyModelMeta>) -> ModelManager {
        let first_id = models.first().map(|m| m.id.clone());
        let mut reg = empty_registry();
        for m in models {
            reg.register(m);
        }
        let mut mm = ModelManager::new(reg, fake_builder());
        if let Some(id) = first_id {
            mm.select_model(&id).unwrap();
        }
        mm
    }

    #[test]
    fn register_without_select_leaves_unset() {
        let mut reg = empty_registry();
        reg.register(meta("m1", true, &["off", "high"]));
        let mm = ModelManager::new(reg, fake_builder());
        assert!(mm.current_model().is_none());
        assert_eq!(mm.current_index(), None);
    }

    #[test]
    fn new_model_manager_empty_registry() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert!(mm.current_model().is_none());
        assert_eq!(mm.current_index(), None);
        assert_eq!(mm.thinking_level(), ThinkingLevel::Off);
    }

    #[test]
    fn new_model_manager_default_thinking() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert_eq!(mm.thinking_level, ThinkingLevel::Medium);
    }

    #[test]
    fn set_thinking_level_without_model_ok() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        mm.set_thinking_level(ThinkingLevel::Low).unwrap();
        assert_eq!(mm.thinking_level, ThinkingLevel::Low);
    }

    #[test]
    fn set_thinking_level_rejects_unsupported() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "high"])]);
        // select_model clamps Medium → High for this support set
        assert_eq!(mm.thinking_level(), ThinkingLevel::High);
        assert!(mm.set_thinking_level(ThinkingLevel::Xhigh).is_err());
        assert_eq!(mm.thinking_level, ThinkingLevel::High);
        mm.set_thinking_level(ThinkingLevel::Off).unwrap();
        assert_eq!(mm.thinking_level(), ThinkingLevel::Off);
        mm.set_thinking_level(ThinkingLevel::High).unwrap();
        assert_eq!(mm.thinking_level(), ThinkingLevel::High);
    }

    #[test]
    fn clamp_on_switch_from_xhigh() {
        let mut mm = manager_with(vec![
            meta("wide", true, &["off", "high", "xhigh"]),
            meta("narrow", true, &["off", "high"]),
        ]);
        mm.set_thinking_level(ThinkingLevel::Xhigh).unwrap();
        mm.select_model("narrow").unwrap();
        assert_ne!(mm.thinking_level(), ThinkingLevel::Xhigh);
        assert!(matches!(
            mm.thinking_level(),
            ThinkingLevel::Off | ThinkingLevel::High | ThinkingLevel::Medium
        ));
        assert_eq!(mm.thinking_level(), ThinkingLevel::High);
    }

    #[test]
    fn preferred_default_used_when_clamping() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "low", "high"])]);
        mm.thinking_level = ThinkingLevel::Xhigh;
        mm.set_preferred_default(Some(ThinkingLevel::Low));
        mm.clamp_thinking_to_model();
        assert_eq!(mm.thinking_level(), ThinkingLevel::Low);
    }

    #[test]
    fn cycle_thinking_level_wraps() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "high"])]);
        mm.set_thinking_level(ThinkingLevel::Off).unwrap();
        assert_eq!(mm.cycle_thinking_level().unwrap(), ThinkingLevel::High);
        assert_eq!(mm.cycle_thinking_level().unwrap(), ThinkingLevel::Off);
    }

    /// c1165: after cycle/set, the same options path ReAct uses MUST resolve to
    /// the matching OpenAI `reasoning_effort` (or Omit when Off).
    #[test]
    fn cycle_then_resolve_openai_effort_matches_level() {
        use crate::domain::types::{
            ResolvedThinking, ThinkingAdapterKind, resolve_thinking_for_request,
        };

        let mut mm = manager_with(vec![meta("m1", true, &["off", "medium", "high"])]);
        mm.set_thinking_level(ThinkingLevel::Off).unwrap();

        let off = resolve_thinking_for_request(
            mm.thinking_level(),
            mm.current_model().map(|m| &m.thinking_level_map),
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(off, ResolvedThinking::Omit);

        assert_eq!(mm.cycle_thinking_level().unwrap(), ThinkingLevel::Medium);
        let mid = resolve_thinking_for_request(
            mm.thinking_level(),
            mm.current_model().map(|m| &m.thinking_level_map),
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(mid, ResolvedThinking::OpenAiEffort("medium".into()));

        assert_eq!(mm.cycle_thinking_level().unwrap(), ThinkingLevel::High);
        let high = resolve_thinking_for_request(
            mm.thinking_level(),
            mm.current_model().map(|m| &m.thinking_level_map),
            None,
            ThinkingAdapterKind::OpenAi,
        );
        assert_eq!(high, ResolvedThinking::OpenAiEffort("high".into()));
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
