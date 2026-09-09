//! Model / thinking selection methods on [`AgentCapabilities`].

use std::sync::{Arc, Mutex};

use crate::agent::model::manager::ModelManager;
use crate::protocol::error::XyError;
use crate::protocol::model::XyModelMeta;
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

    /// Selected thinking level, including a sticky out-of-set restored value.
    pub fn thinking_level(&self) -> String {
        self.with_models(|mm| mm.thinking_level())
    }

    /// Selected binding when idle (no in-flight turn). Live fixed zone uses the
    /// run coordinator via [`crate::agent::runtime::AgentRuntime`].
    pub fn selected_turn_binding(&self) -> Option<ActiveTurnBinding> {
        self.with_models(ActiveTurnBinding::from_manager)
    }

    /// True while a root turn is live (coordinator probe).
    pub fn has_active_turn(&self) -> bool {
        (self.midturn_active)()
    }

    /// Set thinking level.
    pub async fn set_thinking_level(&mut self, level: String) -> Result<(), XyError> {
        let previous = self.thinking_level();
        self.with_models_mut(|mm| mm.set_thinking_level(level.clone()))?;
        // Persist only a real change: attach-time restore of the same level must
        // not append a parent-less thinkingLevelChange row at the session tail.
        // The select hook still observes every action (thw6).
        if previous != level {
            self.persist_thinking_level_change(previous, level).await;
        } else if let Some(bus) = self.hook_bus.clone() {
            observe_hook_sync(
                &bus,
                "thinking_level_select",
                "",
                serde_json::json!({
                    "level": level,
                    "previous": previous,
                }),
            );
        }
        Ok(())
    }

    async fn persist_thinking_level_change(&self, previous: String, level: String) {
        if let Some(ref sid) = self.session_id {
            let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
                base: EntryBase {
                    entry_type: "thinking_level_change".into(),
                    id: String::new(),
                    parent_id: None,
                    timestamp: 0,
                },
                thinking_level: level.clone(),
            });
            if let Err(error) = self.store.append_session_entry(sid, &entry).await {
                log::warn!(target: "xylitol::session", "persist thinking level change failed: {error}");
            }
        }
        if let Some(bus) = self.hook_bus.clone() {
            observe_hook_sync(
                &bus,
                "thinking_level_select",
                "",
                serde_json::json!({
                    "level": level,
                    "previous": previous,
                }),
            );
        }
    }

    /// Apply Settings `default_thinking_level` for initial session assembly.
    pub fn apply_default_thinking_level(&mut self, raw: Option<&str>) {
        let preferred = raw.map(str::to_owned);
        self.with_models_mut(|mm| {
            mm.set_preferred_default(preferred);
            mm.apply_preferred_or_last();
        });
    }

    /// Apply Settings `thinkingBudgets` (Anthropic budget overrides) for generate.
    pub fn set_thinking_budgets(
        &mut self,
        budgets: Option<crate::protocol::model::ThinkingBudgets>,
    ) {
        self.with_models_mut(|mm| mm.set_thinking_budgets(budgets));
    }

    /// Restore a persisted session level verbatim, without emitting a new entry.
    pub(crate) fn restore_thinking_level(&mut self, level: String) {
        self.with_models_mut(|mm| mm.restore_thinking_level(level));
    }

    /// Select a specific model by ID (`source` = `"set"`).
    pub async fn select_model(&mut self, model_id: &str) -> Result<(), XyError> {
        self.select_model_with_source(model_id, "set").await
    }

    /// Select a model and emit `model_select` with the given source (`set` | `cycle`).
    ///
    /// `source = "restore"` is composition-root assembly (attach-time default
    /// binding): it persists nothing — the session already records its model.
    pub async fn select_model_with_source(
        &mut self,
        model_id: &str,
        source: &str,
    ) -> Result<(), XyError> {
        let previous = self.current_model().map(|m| m.id.clone());
        self.with_models_mut(|mm| mm.select_model(model_id))?;
        // Persist only a real user-driven change: attach-time default restore
        // must not append a parent-less modelChange row that breaks resume
        // projection. The select hook still observes every action (thw6).
        if source != "restore"
            && previous.as_deref() != Some(model_id)
            && let Some(ref sid) = self.session_id
        {
            let entry = SessionEntry::ModelChange(ModelChangeEntry {
                base: EntryBase {
                    entry_type: "model_change".into(),
                    id: String::new(),
                    parent_id: None,
                    timestamp: 0,
                },
                provider: model_id.to_string(),
                model_id: model_id.to_string(),
            });
            if let Err(error) = self.store.append_session_entry(sid, &entry).await {
                log::warn!(target: "xylitol::session", "persist model change failed: {error}");
            }
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
