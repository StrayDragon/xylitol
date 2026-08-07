//! Model / thinking selection methods on [`AgentCapabilities`].

use std::sync::{Arc, Mutex};

use crate::agent::model::manager::ModelManager;
use crate::protocol::error::XyError;
use crate::protocol::model::{ThinkingLevel, XyModelMeta};
use crate::protocol::ports::XyModel;
use crate::protocol::session::{
    EntryBase, ModelChangeEntry, SessionEntry, ThinkingLevelChangeEntry,
};

use super::{ActiveTurnBinding, AgentCapabilities, observe_hook_sync};

impl AgentCapabilities {
    pub(crate) fn with_models<R>(&self, f: impl FnOnce(&ModelManager) -> R) -> R {
        let guard = crate::utils::lock_mutex(&self.model_manager);
        f(&guard)
    }

    pub(crate) fn with_models_mut<R>(&self, f: impl FnOnce(&mut ModelManager) -> R) -> R {
        let mut guard = crate::utils::lock_mutex(&self.model_manager);
        f(&mut guard)
    }

    /// Shared handle for ReAct NextTurn refresh (clone into the run stream).
    pub(crate) fn model_manager_handle(&self) -> Arc<Mutex<ModelManager>> {
        self.model_manager.clone()
    }

    /// Get the currently selected model metadata (clone for lock safety).
    pub fn current_model(&self) -> Option<XyModelMeta> {
        self.with_models(|mm| mm.current_model().cloned())
    }

    /// Build the selected model instance.
    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, XyError> {
        self.with_models(|mm| mm.build_current_model())
    }

    /// Selected thinking level (clamped).
    pub fn thinking_level(&self) -> ThinkingLevel {
        self.with_models(|mm| mm.thinking_level())
    }

    /// Selected binding when idle (no in-flight turn). Live chrome uses the
    /// run coordinator via [`crate::agent::runtime::AgentRuntime`].
    pub fn selected_turn_binding(&self) -> Option<ActiveTurnBinding> {
        self.with_models(|mm| {
            let meta = mm.current_model()?;
            let levels = crate::agent::model::manager::ModelManager::levels_for_meta(meta);
            Some(ActiveTurnBinding {
                model_id: meta.id.clone(),
                display_name: if meta.display_name.is_empty() {
                    meta.id.clone()
                } else {
                    meta.display_name.clone()
                },
                thinking: mm.thinking_level(),
                omit_thinking: !ThinkingLevel::is_adjustable(&levels),
            })
        })
    }

    /// True while a root turn is live (coordinator probe).
    pub fn has_active_turn(&self) -> bool {
        (self.midturn_active)()
    }

    /// Set thinking level.
    pub fn set_thinking_level(&mut self, level: ThinkingLevel) -> Result<(), XyError> {
        let previous = self.thinking_level();
        self.with_models_mut(|mm| mm.set_thinking_level(level))?;
        self.persist_thinking_level_change(previous, level);
        Ok(())
    }

    /// Cycle to the next level in the current model's support list.
    pub fn cycle_thinking_level(&mut self) -> Result<ThinkingLevel, XyError> {
        let previous = self.thinking_level();
        let level = self.with_models_mut(|mm| mm.cycle_thinking_level())?;
        self.persist_thinking_level_change(previous, level);
        Ok(level)
    }

    fn persist_thinking_level_change(&self, previous: ThinkingLevel, level: ThinkingLevel) {
        // Fire-and-forget persistence via the session store port.
        if let Some(ref sid) = self.session_id {
            let store = self.store.clone();
            let sid = sid.clone();
            let level_str = level.as_str().to_string();
            tokio::spawn(async move {
                let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
                    base: EntryBase {
                        entry_type: "thinking_level_change".into(),
                        id: String::new(),
                        parent_id: None,
                        timestamp: String::new(),
                    },
                    thinking_level: level_str,
                });
                let _ = store.append_session_entry(&sid, &entry).await;
            });
        }
        if let Some(bus) = self.hook_bus.clone() {
            observe_hook_sync(
                &bus,
                "thinking_level_select",
                "",
                serde_json::json!({
                    "level": level.as_str(),
                    "previous": previous.as_str(),
                }),
            );
        }
    }

    /// Apply Settings `default_thinking_level` (if parseable) then preferred-or-highest.
    pub fn apply_default_thinking_level(&mut self, raw: Option<&str>) {
        let preferred = raw.and_then(ThinkingLevel::parse);
        self.with_models_mut(|mm| {
            mm.set_preferred_default(preferred);
            mm.apply_preferred_or_highest();
        });
    }

    /// Select a specific model by ID (`source` = `"set"`).
    pub fn select_model(&mut self, model_id: &str) -> Result<(), XyError> {
        self.select_model_with_source(model_id, "set")
    }

    /// Select a model and emit `model_select` with the given source (`set` | `cycle`).
    pub fn select_model_with_source(
        &mut self,
        model_id: &str,
        source: &str,
    ) -> Result<(), XyError> {
        let previous = self.current_model().map(|m| m.id.clone());
        self.with_models_mut(|mm| mm.select_model(model_id))?;
        // Fire-and-forget persistence via the session store port.
        if let Some(ref sid) = self.session_id {
            let store = self.store.clone();
            let sid = sid.clone();
            let mid = model_id.to_string();
            tokio::spawn(async move {
                let entry = SessionEntry::ModelChange(ModelChangeEntry {
                    base: EntryBase {
                        entry_type: "model_change".into(),
                        id: String::new(),
                        parent_id: None,
                        timestamp: String::new(),
                    },
                    provider: mid.clone(),
                    model_id: mid,
                });
                let _ = store.append_session_entry(&sid, &entry).await;
            });
        }
        if let Some(bus) = self.hook_bus.clone() {
            observe_hook_sync(
                &bus,
                "model_select",
                "",
                serde_json::json!({
                    "model": model_id,
                    "previous": previous,
                    "source": source,
                }),
            );
        }
        Ok(())
    }
}
