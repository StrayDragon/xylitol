//! Task-scoped model resolution — single entry for compaction summary and future 旁路任务.

use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use crate::agent::model::manager::ModelManager;
use crate::protocol::error::XyError;
use crate::protocol::model::{THINKING_OFF, XyModelMeta, last_declared_thinking_level};
use crate::protocol::model_entry::XyModelEntryConfig;
use crate::protocol::ports::{XyGenerateOptions, XyModel};

/// Attribution for compaction summary model selection (obs + notice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionModelAttribution {
    pub requested_model: Option<String>,
    pub actual_model: String,
    pub fallback: bool,
    pub thinking_rejected: bool,
}

impl CompactionModelAttribution {
    pub fn enrich_obs(
        &self,
        obs: &xylitol_ai_bridge::ObsSessionContext,
    ) -> xylitol_ai_bridge::ObsSessionContext {
        let mut out = obs.clone();
        out.compaction_requested_model = self.requested_model.clone();
        out.compaction_actual_model = Some(self.actual_model.clone());
        out.compaction_model_fallback = Some(self.fallback);
        out.compaction_thinking_rejected = Some(self.thinking_rejected);
        out
    }

    pub fn notice_message(&self) -> Option<String> {
        if self.fallback {
            let requested = self
                .requested_model
                .as_deref()
                .unwrap_or("configured summary model");
            Some(format!(
                "Compaction summary fell back to {} (could not use {})",
                self.actual_model, requested
            ))
        } else {
            None
        }
    }
}

/// Resolved compaction summary model + generate options.
#[derive(Clone)]
pub struct CompactionSummaryBinding {
    pub model: Arc<dyn XyModel>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub meta: XyModelMeta,
    pub generate_options: XyGenerateOptions,
    pub attribution: CompactionModelAttribution,
}

impl CompactionSummaryBinding {
    /// Test/BDD helper: wrap a built model without task-entry resolution.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn for_test(model: Arc<dyn XyModel>, wire_id: &str) -> Self {
        use crate::protocol::model::{XyModelConfig, XyModelKind, XyModelMeta};
        let meta = XyModelMeta {
            id: wire_id.into(),
            config: XyModelConfig {
                kind: XyModelKind::Fake,
                api_key: String::new(),
                model: wire_id.into(),
                base_url: None,
                api: None,
                compat: None,
            },
            display_name: wire_id.into(),
            thinking: false,
            context_window: 128_000,
            api: String::new(),
            provider: "fake".into(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: vec![THINKING_OFF.into()],
            thinking_level_map: Default::default(),
        };
        Self {
            model,
            meta,
            generate_options: XyGenerateOptions::default(),
            attribution: CompactionModelAttribution {
                requested_model: None,
                actual_model: wire_id.into(),
                fallback: false,
                thinking_rejected: false,
            },
        }
    }
}

/// Resolve the model and thinking options for a compaction summary request.
pub fn resolve_compaction_summary(
    settings: &CompactionSettings,
    mm: &ModelManager,
    obs_session: &xylitol_ai_bridge::ObsSessionContext,
) -> Result<CompactionSummaryBinding, XyError> {
    let current_meta = mm
        .current_model()
        .ok_or_else(|| XyError::Config("no model configured".into()))?;
    let (model, meta, requested, fallback) = if let Some(entry) = settings.model.as_ref() {
        match try_build_task_model(entry, mm) {
            Ok((built, ephemeral)) => (built, ephemeral, Some(entry.model.clone()), false),
            Err(e) => {
                log::warn!(
                    "compaction task model build failed: {e}; falling back to current model"
                );
                let built = mm.build_current_model()?;
                (built, current_meta.clone(), Some(entry.model.clone()), true)
            }
        }
    } else {
        let built = mm.build_current_model()?;
        (built, current_meta.clone(), None, false)
    };

    let levels = ModelManager::levels_for_meta(&meta);
    let inherited = last_declared_thinking_level(&levels);
    let (thinking_level, thinking_rejected) =
        resolve_summary_thinking(settings.thinking_level.as_deref(), &levels, &inherited);

    if thinking_rejected {
        log::warn!(
            "compaction thinking_level `{}` not in support set {:?}; using inherited `{}`",
            settings.thinking_level.as_deref().unwrap_or(""),
            levels,
            thinking_level
        );
    }

    let attribution = CompactionModelAttribution {
        requested_model: requested,
        actual_model: meta.config.model.clone(),
        fallback,
        thinking_rejected,
    };

    let obs = attribution.enrich_obs(obs_session);
    let generate_options = XyGenerateOptions {
        thinking_level,
        level_map: meta.thinking_level_map.clone(),
        thinking_budgets: mm.thinking_budgets().cloned(),
        system_prompt: None,
        max_output_tokens: None,
        obs_parent: None,
        obs_session: obs,
    };

    Ok(CompactionSummaryBinding {
        model,
        meta,
        generate_options,
        attribution,
    })
}

fn try_build_task_model(
    entry: &XyModelEntryConfig,
    mm: &ModelManager,
) -> Result<(Arc<dyn XyModel>, XyModelMeta), XyError> {
    let meta = entry.to_ephemeral_meta().map_err(XyError::Config)?;
    let built = (mm.model_builder)(&meta.config);
    Ok((built, meta))
}

fn resolve_summary_thinking(
    override_level: Option<&str>,
    levels: &[String],
    inherited: &str,
) -> (String, bool) {
    match override_level {
        None => (inherited.to_string(), false),
        Some(level) if levels.iter().any(|supported| supported == level) => {
            (level.to_string(), false)
        }
        Some(_) => (inherited.to_string(), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::model::registry::ModelRegistry;
    use crate::protocol::model::{XyModelConfig, XyModelKind};

    type ModelBuilderFn = crate::protocol::ports::XyModelBuilder;

    fn fake_builder() -> ModelBuilderFn {
        Arc::new(crate::infra::provider::factory::build_provider)
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
            context_window: 128_000,
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
        let mut reg = ModelRegistry::new();
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
    fn unconfigured_inherits_last_declared_thinking() {
        let mm = manager_with(vec![meta("main", true, &["off", "high"])]);
        let settings = CompactionSettings::default();
        let binding = resolve_compaction_summary(&settings, &mm, &Default::default()).unwrap();
        assert_eq!(binding.generate_options.thinking_level, "high");
        assert!(!binding.attribution.fallback);
        assert_eq!(binding.attribution.actual_model, "main");
    }

    fn invalid_task_entry() -> XyModelEntryConfig {
        XyModelEntryConfig {
            provider: XyModelKind::Fake,
            model: "bad-summary".into(),
            thinking: true,
            thinking_levels: Some(vec!["".into()]),
            ..Default::default()
        }
    }

    #[test]
    fn task_entry_builds_independent_instance() {
        let mm = manager_with(vec![meta("main", true, &["off", "high"])]);
        let count_before = mm.registry.len();
        let ids_before: Vec<_> = mm.registry.list().iter().map(|m| m.id.clone()).collect();
        let settings = CompactionSettings {
            model: Some(XyModelEntryConfig {
                provider: XyModelKind::Fake,
                model: "summary-bot".into(),
                thinking: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let binding = resolve_compaction_summary(&settings, &mm, &Default::default()).unwrap();
        assert_eq!(binding.meta.config.model, "summary-bot");
        assert_eq!(binding.attribution.actual_model, "summary-bot");
        assert!(!binding.attribution.fallback);
        assert_eq!(binding.generate_options.thinking_level, THINKING_OFF);
        assert_eq!(mm.registry.len(), count_before);
        assert_eq!(
            mm.registry
                .list()
                .iter()
                .map(|m| m.id.clone())
                .collect::<Vec<_>>(),
            ids_before
        );
    }

    #[test]
    fn task_build_failure_falls_back_with_attribution() {
        let mm = manager_with(vec![meta("main", true, &["off", "high"])]);
        let settings = CompactionSettings {
            model: Some(invalid_task_entry()),
            ..Default::default()
        };
        let binding = resolve_compaction_summary(&settings, &mm, &Default::default()).unwrap();
        assert_eq!(binding.attribution.actual_model, "main");
        assert!(binding.attribution.fallback);
        assert!(binding.attribution.notice_message().is_some());
    }

    #[test]
    fn thinking_override_hit_and_miss() {
        let mm = manager_with(vec![meta("main", true, &["off", "high"])]);
        let hit = CompactionSettings {
            thinking_level: Some("off".into()),
            ..Default::default()
        };
        let binding = resolve_compaction_summary(&hit, &mm, &Default::default()).unwrap();
        assert_eq!(binding.generate_options.thinking_level, "off");
        assert!(!binding.attribution.thinking_rejected);

        let miss = CompactionSettings {
            thinking_level: Some("max".into()),
            ..Default::default()
        };
        let binding = resolve_compaction_summary(&miss, &mm, &Default::default()).unwrap();
        assert_eq!(binding.generate_options.thinking_level, "high");
        assert!(binding.attribution.thinking_rejected);
    }

    #[test]
    fn undeclared_thinking_levels_inherit_off() {
        let mm = manager_with(vec![meta("plain", false, &[])]);
        let binding =
            resolve_compaction_summary(&CompactionSettings::default(), &mm, &Default::default())
                .unwrap();
        assert_eq!(binding.generate_options.thinking_level, THINKING_OFF);
    }
}
