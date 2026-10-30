use std::sync::Arc;

use crate::agent::model::registry::ModelRegistry;
use xylitol_plugin_sdk::XyModel;
use xylitol_plugin_sdk::error::XyError;
use xylitol_plugin_sdk::model::{
    THINKING_OFF, ThinkingBudgets, XyModelMeta, last_declared_thinking_level,
};

pub struct ModelManager {
    pub(crate) registry: ModelRegistry,
    pub(crate) current_index: Option<usize>,
    pub(crate) thinking_level: String,
    preferred_default: Option<String>,
    thinking_budgets: Option<ThinkingBudgets>,
    pub(crate) model_builder: xylitol_plugin_sdk::XyModelBuilder,
}

impl ModelManager {
    pub fn new(registry: ModelRegistry, model_builder: xylitol_plugin_sdk::XyModelBuilder) -> Self {
        Self {
            registry,
            current_index: None,
            thinking_level: THINKING_OFF.into(),
            preferred_default: None,
            thinking_budgets: None,
            model_builder,
        }
    }

    pub fn set_preferred_default(&mut self, level: Option<String>) {
        self.preferred_default = level;
    }

    pub fn set_thinking_budgets(&mut self, budgets: Option<ThinkingBudgets>) {
        self.thinking_budgets = budgets;
    }

    pub fn thinking_budgets(&self) -> Option<&ThinkingBudgets> {
        self.thinking_budgets.as_ref()
    }

    pub fn current_model(&self) -> Option<&XyModelMeta> {
        let idx = self.current_index?;
        self.registry.list().get(idx)
    }

    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, XyError> {
        let meta = self
            .current_model()
            .ok_or_else(|| XyError::Config("no model configured".into()))?;
        Ok((self.model_builder)(&meta.config))
    }

    pub fn supported_levels(&self) -> Option<Vec<String>> {
        let meta = self.current_model()?;
        Some(Self::levels_for_meta(meta))
    }

    pub fn levels_for_meta(meta: &XyModelMeta) -> Vec<String> {
        if !meta.thinking || meta.thinking_levels.is_empty() {
            return vec![THINKING_OFF.into()];
        }
        meta.thinking_levels.clone()
    }

    pub fn thinking_level(&self) -> String {
        self.thinking_level.clone()
    }

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

    pub(crate) fn restore_thinking_level(&mut self, level: String) {
        self.thinking_level = level;
    }

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

    pub fn default_thinking_for_current(&mut self) {
        let Some(levels) = self.supported_levels() else {
            self.thinking_level = THINKING_OFF.into();
            return;
        };
        self.thinking_level = last_declared_thinking_level(&levels);
    }

    pub fn select_model(&mut self, model_id: &str) -> Result<(), XyError> {
        let model = self
            .registry
            .find(model_id)
            .ok_or_else(|| XyError::Config(format!("model not found: {model_id}")))?;
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

    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::ModelManager;
    use crate::agent::model::registry::ModelRegistry;
    use xylitol_plugin_sdk::error::XyError;
    use xylitol_plugin_sdk::model::XyModelMeta;
    use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind};

    type ModelBuilderFn = xylitol_plugin_sdk::XyModelBuilder;

    fn empty_registry() -> ModelRegistry {
        ModelRegistry::new()
    }

    fn fake_builder() -> ModelBuilderFn {
        crate::test_support::dummy_model_builder()
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
                prompt_cache: None,
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
        mm.set_thinking_budgets(Some(xylitol_plugin_sdk::model::ThinkingBudgets {
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
    }

    #[test]
    fn new_model_manager_empty_registry() {
        let mm = ModelManager::new(empty_registry(), fake_builder());
        assert!(mm.current_model().is_none());
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
    fn restored_out_of_set_level_stays_sticky() {
        let mut mm = manager_with(vec![
            meta("wide", true, &["off", "high", "vendor-max"]),
            meta("narrow", true, &["off", "high"]),
        ]);
        mm.set_thinking_level("vendor-max".into()).unwrap();
        mm.select_model("narrow").unwrap();
        assert_eq!(mm.thinking_level(), "high");
        mm.restore_thinking_level("vendor-max".into());
        assert_eq!(mm.thinking_level(), "vendor-max");
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
