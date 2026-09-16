//! SettingsManager — three-tier deep merge, reload, lock-based persistence.

use std::sync::{Arc, Mutex};

use super::storage::{
    FileSettingsStorage, InMemorySettingsStorage, SettingsScope, SettingsStorage,
};
use super::types::*;

/// Manages global and project settings with deep merge.
///
/// Product path only consumes the merged [`Self::settings`] view (bootstrap);
/// the backing fields feed the BDD @executable settings surface (in_memory /
/// trust queries).
#[allow(dead_code)] // backing store of BDD @executable settings surface (c2750)
pub struct SettingsManager {
    storage: Box<dyn SettingsStorage>,
    global_settings: Settings,
    project_settings: Settings,
    /// Merged view: project overrides global.
    pub settings: Settings,
    project_trusted: bool,
    global_error: Option<String>,
    project_error: Option<String>,
    errors: Arc<Mutex<Vec<(SettingsScope, String)>>>,
}

impl SettingsManager {
    // ── Construction ────────────────────────────────────────────

    /// Create from file storage (global ~/.xylitol/settings.json + project `<cwd>`/.xylitol/settings.json).
    pub fn from_files(
        cwd: &std::path::Path,
        agent_dir: &std::path::Path,
        project_trusted: bool,
    ) -> Self {
        let storage = FileSettingsStorage::new(cwd, agent_dir);
        Self::from_storage(Box::new(storage), project_trusted)
    }

    /// Create from an arbitrary storage backend (in-memory for tests).
    pub fn from_storage(storage: Box<dyn SettingsStorage>, project_trusted: bool) -> Self {
        let global = Self::load_from_storage(storage.as_ref(), SettingsScope::Global, true);
        let project =
            Self::load_from_storage(storage.as_ref(), SettingsScope::Project, project_trusted);
        let merged = Self::deep_merge(&global.0, &project.0);

        let mut errors = Vec::new();
        if let Some(ref e) = global.1 {
            errors.push((SettingsScope::Global, e.clone()));
        }
        if let Some(ref e) = project.1 {
            errors.push((SettingsScope::Project, e.clone()));
        }

        Self {
            storage,
            global_settings: global.0,
            project_settings: project.0,
            settings: merged,
            project_trusted,
            global_error: global.1,
            project_error: project.1,
            errors: Arc::new(Mutex::new(errors)),
        }
    }

    /// Create an in-memory manager from partial settings.
    /// BDD @executable contract (`设置` steps); no product caller today.
    #[allow(dead_code)] // BDD @executable contract, not product-called
    pub fn in_memory(settings: Settings) -> Self {
        let storage = Box::new(InMemorySettingsStorage::default());
        let json = serde_json::to_string(&settings).unwrap();
        storage.with_lock(SettingsScope::Global, &mut |_cur| Some(json.clone()));
        Self::from_storage(storage, true)
    }

    fn load_from_storage(
        storage: &dyn SettingsStorage,
        scope: SettingsScope,
        trusted: bool,
    ) -> (Settings, Option<String>) {
        if matches!(scope, SettingsScope::Project) && !trusted {
            return (Settings::default(), None);
        }

        let mut result = None;
        let mut error = None;

        storage.with_lock(scope, &mut |current| {
            match current {
                Some(content) if !content.is_empty() => {
                    match serde_json::from_str::<Settings>(content) {
                        Ok(s) => result = Some(s),
                        Err(e) => {
                            error = Some(format!("parse settings {scope:?}: {e}"));
                        }
                    }
                }
                _ => {}
            }
            None // don't modify
        });

        (result.unwrap_or_default(), error)
    }

    // ── Deep merge ────────────────────────────────────────────

    /// Deep-merge two Settings structs: overrides win, None fields skip.
    /// Nested objects (compaction, thinking_budgets) merge field-by-field.
    pub fn deep_merge(base: &Settings, overrides: &Settings) -> Settings {
        let mut result = base.clone();

        if overrides.default_thinking_level.is_some() {
            result.default_thinking_level = overrides.default_thinking_level.clone();
        }
        if overrides.steering_mode.is_some() {
            result.steering_mode = overrides.steering_mode.clone();
        }
        if overrides.follow_up_mode.is_some() {
            result.follow_up_mode = overrides.follow_up_mode.clone();
        }

        merge_compaction(
            &mut result.compaction,
            base.compaction.as_ref(),
            overrides.compaction.as_ref(),
        );
        merge_thinking_budgets(
            &mut result.thinking_budgets,
            base.thinking_budgets.as_ref(),
            overrides.thinking_budgets.as_ref(),
        );

        result
    }

    // ── Accessors (with defaults) ────────────────────────────

    pub fn get_thinking_budgets(&self) -> Option<&ThinkingBudgets> {
        self.settings.thinking_budgets.as_ref()
    }

    pub fn get_steering_mode(&self) -> SteeringMode {
        self.settings.steering_mode.clone().unwrap_or_default()
    }

    pub fn get_follow_up_mode(&self) -> SteeringMode {
        self.settings.follow_up_mode.clone().unwrap_or_default()
    }

    // ── Mutators (global scope, persist immediately) ──────────

    // ── Project trust ─────────────────────────────────────────

    /// BDD @executable contract (`settings 信任` steps); no product caller today.
    #[allow(dead_code)] // BDD @executable contract, not product-called
    pub fn is_project_trusted(&self) -> bool {
        self.project_trusted
    }

    // ── Raw access for callers that need structure ────────────

    pub fn get_settings(&self) -> &Settings {
        &self.settings
    }

    /// BDD @executable contract (`settings 信任` steps); no product caller today.
    #[allow(dead_code)] // BDD @executable contract, not product-called
    pub fn get_project_settings(&self) -> &Settings {
        &self.project_settings
    }
}

// ── Private: nested merge helpers ─────────────────────────────────

fn merge_compaction(
    target: &mut Option<XyCompactionSettingsConfig>,
    base: Option<&XyCompactionSettingsConfig>,
    overrides: Option<&XyCompactionSettingsConfig>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(XyCompactionSettingsConfig {
        enabled: o.enabled.or(b.enabled),
        reserve_tokens: o.reserve_tokens.or(b.reserve_tokens),
        keep_recent_tokens: o.keep_recent_tokens.or(b.keep_recent_tokens),
        // rc15: task model + thinking override are config.yaml-only.
        model: None,
        thinking_level: None,
    });
}

fn merge_thinking_budgets(
    target: &mut Option<ThinkingBudgets>,
    base: Option<&ThinkingBudgets>,
    overrides: Option<&ThinkingBudgets>,
) {
    let b = match base {
        Some(b) => b,
        None => {
            *target = overrides.cloned();
            return;
        }
    };
    let o = match overrides {
        Some(o) => o,
        None => {
            *target = Some(b.clone());
            return;
        }
    };
    *target = Some(ThinkingBudgets {
        minimal: o.minimal.or(b.minimal),
        low: o.low.or(b.low),
        medium: o.medium.or(b.medium),
        high: o.high.or(b.high),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_merge_override() {
        let base = Settings {
            compaction: Some(XyCompactionSettingsConfig {
                enabled: Some(true),
                reserve_tokens: Some(16384),
                keep_recent_tokens: Some(20000),
                model: None,
                thinking_level: None,
            }),
            ..Default::default()
        };
        let overrides = Settings {
            compaction: Some(XyCompactionSettingsConfig {
                enabled: Some(false),
                reserve_tokens: None,
                keep_recent_tokens: None,
                model: Some(crate::protocol::model_entry::XyModelEntryConfig {
                    provider: crate::protocol::model::XyModelKind::Fake,
                    model: "ignored".into(),
                    ..Default::default()
                }),
                thinking_level: Some("high".into()),
            }),
            ..Default::default()
        };

        let merged = SettingsManager::deep_merge(&base, &overrides);
        let comp = merged.compaction.as_ref().unwrap();
        assert_eq!(comp.enabled, Some(false));
        assert_eq!(comp.reserve_tokens, Some(16384));
        assert_eq!(comp.keep_recent_tokens, Some(20000));
        assert!(comp.model.is_none(), "settings merge must drop model");
        assert!(
            comp.thinking_level.is_none(),
            "settings merge must drop thinking_level"
        );
    }

    #[test]
    fn test_deep_merge_keeps_base_thinking_level() {
        let base = Settings {
            default_thinking_level: Some("high".into()),
            ..Default::default()
        };
        let project = Settings::default();
        let merged = SettingsManager::deep_merge(&base, &project);
        assert_eq!(merged.default_thinking_level.as_deref(), Some("high"));
    }

    #[test]
    fn test_in_memory_roundtrip() {
        let mgr = SettingsManager::in_memory(Settings {
            default_thinking_level: Some("medium".into()),
            ..Default::default()
        });
        assert_eq!(
            mgr.get_settings().default_thinking_level.as_deref(),
            Some("medium")
        );
    }

    #[test]
    fn test_steering_defaults() {
        let mgr = SettingsManager::in_memory(Default::default());
        assert_eq!(mgr.get_steering_mode(), SteeringMode::OneAtATime);
        assert_eq!(mgr.get_follow_up_mode(), SteeringMode::OneAtATime);
    }
}
