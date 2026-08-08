//! ModelManager — model registry, selection, and thinking level.
//!
//! Extracted from [`AgentCapabilities`](crate::agent::capabilities::AgentCapabilities) to isolate
//! model-related responsibilities into a focused component.

use std::sync::Arc;

use crate::agent::model::registry::ModelRegistry;
use crate::protocol::error::XyError;
use crate::protocol::model::{
    THINKING_OFF, ThinkingBudgets, XyModelConfig, XyModelMeta, last_declared_thinking_level,
    thinking_levels_are_adjustable,
};
use crate::protocol::ports::XyModel;

/// Manages model registry, current model selection, and thinking level.
///
/// Owned by [`AgentCapabilities`](crate::agent::capabilities::AgentCapabilities) as a composed field.
/// The provider is built via an injected `model_builder` (the agent layer must
/// not call `infra::provider::factory::build_provider` directly; the composition
/// root supplies the builder).
pub struct ModelManager {
    /// Model registry for provider/model lookup.
    pub(crate) registry: ModelRegistry,
    /// Index of the currently selected model in the registry (`None` = unset).
    pub(crate) current_index: Option<usize>,
    /// Current session level. This is intentionally a raw, vendor-declared string:
    /// a restored level may remain sticky after the configuration support set changes.
    pub(crate) thinking_level: String,
    /// Preferred default from Settings (`default_thinking_level`), if any.
    preferred_default: Option<String>,
    /// Optional Settings Anthropic budget overrides for known level names.
    thinking_budgets: Option<ThinkingBudgets>,
    /// Injected provider factory (composition-root-supplied).
    pub(crate) model_builder: crate::protocol::ports::XyModelBuilder,
}

impl ModelManager {
    /// Create a new ModelManager with the given registry, default index, and
    /// injected provider builder.
    pub fn new(
        registry: ModelRegistry,
        model_builder: crate::protocol::ports::XyModelBuilder,
    ) -> Self {
        Self {
            registry,
            current_index: None,
            thinking_level: THINKING_OFF.into(),
            preferred_default: None,
            thinking_budgets: None,
            model_builder,
        }
    }

    /// Store Settings `default_thinking_level` for initial session assembly.
    pub fn set_preferred_default(&mut self, level: Option<String>) {
        self.preferred_default = level;
    }

    /// Store Settings `thinkingBudgets` for Anthropic (and mapped) budget resolve.
    pub fn set_thinking_budgets(&mut self, budgets: Option<ThinkingBudgets>) {
        self.thinking_budgets = budgets;
    }

    /// Current Settings thinking budgets, if any.
    pub fn thinking_budgets(&self) -> Option<&ThinkingBudgets> {
        self.thinking_budgets.as_ref()
    }

    // ── Current model ────────────────────────────────────────────

    /// Get the currently selected model metadata.
    pub fn current_model(&self) -> Option<&XyModelMeta> {
        let idx = self.current_index?;
        self.registry.list().get(idx)
    }

    /// Build a provider instance from the current model config (via the
    /// injected builder).
    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, XyError> {
        let meta = self
            .current_model()
            .ok_or_else(|| XyError::Config("no model configured".into()))?;
        (self.model_builder)(&meta.config).map_err(|e| XyError::Provider(anyhow::anyhow!(e)))
    }

    // ── Thinking level ───────────────────────────────────────────

    /// Declared levels supported by the current model (`off`-only when disabled).
    pub fn supported_levels(&self) -> Option<Vec<String>> {
        let meta = self.current_model()?;
        Some(Self::levels_for_meta(meta))
    }

    /// Levels for an arbitrary meta (picker / UI).
    pub fn levels_for_meta(meta: &XyModelMeta) -> Vec<String> {
        if !meta.thinking || meta.thinking_levels.is_empty() {
            return vec![THINKING_OFF.into()];
        }
        meta.thinking_levels.clone()
    }

    /// Get the exact current level, including a sticky out-of-set restored value.
    pub fn thinking_level(&self) -> String {
        self.thinking_level.clone()
    }

    /// Set a new thinking level. Rejects if current model has a support set
    /// that does not include `level` (current value unchanged).
    pub fn set_thinking_level(&mut self, level: String) -> Result<(), XyError> {
        let levels = self
            .supported_levels()
            .ok_or_else(|| XyError::Config("no model configured".into()))?;
        if !levels.iter().any(|supported| supported == &level) {
            return Err(XyError::Config(format!(
                "thinking level `{}` is not supported by the current model",
                level
            )));
        }
        self.thinking_level = level;
        Ok(())
    }

    /// Restore an exact session value without validation or persistence.
    pub(crate) fn restore_thinking_level(&mut self, level: String) {
        self.thinking_level = level;
    }

    /// Session-first assembly: Settings default if declared, otherwise the
    /// final configured list item. Never use this after a model switch.
    pub fn apply_preferred_or_last(&mut self) {
        let Some(levels) = self.supported_levels() else {
            self.thinking_level = THINKING_OFF.into();
            return;
        };
        if let Some(default) = self
            .preferred_default
            .as_ref()
            .filter(|default| levels.iter().any(|level| level == *default))
        {
            self.thinking_level = default.clone();
            return;
        }
        self.thinking_level = last_declared_thinking_level(&levels);
    }

    /// Default thinking for a freshly selected model (the declared final item).
    pub fn default_thinking_for_current(&mut self) {
        let Some(levels) = self.supported_levels() else {
            self.thinking_level = THINKING_OFF.into();
            return;
        };
        self.thinking_level = last_declared_thinking_level(&levels);
    }

    /// Cycle to the next level in the current model's support list.
    ///
    /// A sticky out-of-set level lands on the list's final item before normal
    /// cyclic traversal resumes.
    pub fn cycle_thinking_level(&mut self) -> Result<String, XyError> {
        let levels = self
            .supported_levels()
            .ok_or_else(|| XyError::Config("no model configured".into()))?;
        if levels.is_empty() {
            return Err(XyError::Config(
                "current model has no thinking levels".into(),
            ));
        }
        let next = match levels
            .iter()
            .position(|level| level == &self.thinking_level)
        {
            Some(index) => levels[(index + 1) % levels.len()].clone(),
            None => last_declared_thinking_level(&levels),
        };
        self.thinking_level = next.clone();
        Ok(next)
    }

    // ── Model switching ──────────────────────────────────────────

    /// Select a model by its ID. Thinking defaults to the final declared item (m10).
    pub fn select_model(&mut self, model_id: &str) -> Result<(), XyError> {
        let model = self
            .registry
            .find(model_id)
            .ok_or_else(|| XyError::Config(format!("model not found: {model_id}")))?;
        // Find index by identity
        let idx = self
            .registry
            .list()
            .iter()
            .position(|m| std::ptr::eq(m, model))
            .unwrap_or(0);
        self.current_index = Some(idx);
        self.default_thinking_for_current();
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

    /// Whether the selected model has a declared adjustable option.
    pub fn thinking_is_adjustable(&self) -> bool {
        self.supported_levels()
            .is_some_and(|levels| thinking_levels_are_adjustable(&levels))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::ModelManager;
    use crate::agent::model::registry::ModelRegistry;
    use crate::protocol::error::XyError;
    use crate::protocol::model::XyModelMeta;
    use crate::protocol::model::{XyModelConfig, XyModelKind};
    use crate::protocol::ports::XyModel;

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
                compat: None,
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
    fn thinking_budgets_round_trip_on_manager() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "high"])]);
        assert!(mm.thinking_budgets().is_none());
        mm.set_thinking_budgets(Some(crate::protocol::model::ThinkingBudgets {
            minimal: Some(111),
            low: None,
            medium: Some(222),
            high: Some(333),
        }));
        let b = mm.thinking_budgets().expect("budgets");
        assert_eq!(b.minimal, Some(111));
        assert_eq!(b.medium, Some(222));
        assert_eq!(b.high, Some(333));
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
        assert_eq!(mm.thinking_level(), "off");
    }

    #[test]
    fn new_model_manager_default_thinking_is_off() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert_eq!(mm.thinking_level, "off");
    }

    #[test]
    fn set_thinking_level_without_model_is_rejected() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        assert!(mm.set_thinking_level("low".into()).is_err());
        assert_eq!(mm.thinking_level(), "off");
    }

    #[test]
    fn set_thinking_level_rejects_unsupported() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "high"])]);
        assert_eq!(mm.thinking_level(), "high");
        assert!(mm.set_thinking_level("xhigh".into()).is_err());
        assert_eq!(mm.thinking_level(), "high");
        mm.set_thinking_level("off".into()).unwrap();
        assert_eq!(mm.thinking_level(), "off");
        mm.set_thinking_level("high".into()).unwrap();
        assert_eq!(mm.thinking_level(), "high");
    }

    #[test]
    fn set_thinking_level_rejects_case_variant() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "high"])]);

        assert!(mm.set_thinking_level("HIGH".into()).is_err());
        assert_eq!(mm.thinking_level(), "high");
    }

    #[test]
    fn restored_out_of_set_level_stays_sticky_until_cycle() {
        let mut mm = manager_with(vec![
            meta("wide", true, &["off", "high", "vendor-max"]),
            meta("narrow", true, &["off", "high"]),
        ]);
        mm.set_thinking_level("vendor-max".into()).unwrap();
        mm.select_model("narrow").unwrap();
        assert_eq!(mm.thinking_level(), "high");
        mm.restore_thinking_level("vendor-max".into());
        assert_eq!(mm.thinking_level(), "vendor-max");
        assert_eq!(mm.cycle_thinking_level().unwrap(), "high");
    }

    #[test]
    fn preferred_default_only_applies_to_first_session_assembly() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "low", "high"])]);
        mm.set_preferred_default(Some("low".into()));
        mm.apply_preferred_or_last();
        assert_eq!(mm.thinking_level(), "low");
        mm.select_model("m1").unwrap();
        assert_eq!(mm.thinking_level(), "high");
    }

    #[test]
    fn select_model_defaults_to_last_declared_level() {
        let mut mm = manager_with(vec![
            meta("a", true, &["off", "low", "high"]),
            meta("b", true, &["off", "max", "low"]),
        ]);
        mm.set_thinking_level("low".into()).unwrap();
        mm.select_model("b").unwrap();
        assert_eq!(mm.thinking_level(), "low");
    }

    #[test]
    fn select_model_no_thinking_is_off() {
        let mut mm = manager_with(vec![
            meta("think", true, &["off", "high"]),
            meta("plain", false, &[]),
        ]);
        mm.select_model("plain").unwrap();
        assert_eq!(mm.thinking_level(), "off");
    }

    #[test]
    fn cycle_thinking_level_wraps() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "high"])]);
        mm.set_thinking_level("off".into()).unwrap();
        assert_eq!(mm.cycle_thinking_level().unwrap(), "high");
        assert_eq!(mm.cycle_thinking_level().unwrap(), "off");
    }

    #[test]
    fn cycle_preserves_freeform_declared_levels() {
        let mut mm = manager_with(vec![meta("m1", true, &["off", "vendor-mid", "high"])]);
        mm.set_thinking_level("off".into()).unwrap();
        assert_eq!(mm.thinking_level(), "off");

        assert_eq!(mm.cycle_thinking_level().unwrap(), "vendor-mid");
        assert_eq!(mm.thinking_level(), "vendor-mid");

        assert_eq!(mm.cycle_thinking_level().unwrap(), "high");
        assert_eq!(mm.thinking_level(), "high");
    }

    #[test]
    fn select_model_empty_registry_returns_error() {
        let mut mm = ModelManager::new(empty_registry(), fake_builder());
        let result = mm.select_model("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("model not found"),);
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
        let err = result.err().expect("expected error");
        assert!(matches!(
            err,
            XyError::Config(ref s) if s == "no model configured"
        ));
    }
}
