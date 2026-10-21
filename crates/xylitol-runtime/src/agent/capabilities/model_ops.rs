use std::sync::{Arc, Mutex};

use crate::agent::model::manager::ModelManager;
use xylitol_plugin_sdk::error::XyError;
use xylitol_plugin_sdk::model::XyModelMeta;
use xylitol_plugin_sdk::session::{
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

    pub(crate) fn model_manager_handle(&self) -> Arc<Mutex<ModelManager>> {
        self.model_manager.clone()
    }

    pub fn current_model(&self) -> Option<XyModelMeta> {
        self.with_models(|mm| mm.current_model().cloned())
    }

    pub fn fixed_request_context(&self) -> crate::agent::compaction::FixedRequestContext {
        crate::agent::compaction::FixedRequestContext {
            system_prompt: self.system_prompt.clone(),
            tool_schemas: self
                .tools
                .iter()
                .map(|t| xylitol_plugin_sdk::model::XyToolSchema {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    parameters: t.parameters_schema(),
                })
                .collect(),
        }
    }

    pub fn thinking_level(&self) -> String {
        self.with_models(|mm| mm.thinking_level())
    }

    pub fn selected_turn_binding(&self) -> Option<ActiveTurnBinding> {
        self.with_models(ActiveTurnBinding::from_manager)
    }

    pub fn has_active_turn(&self) -> bool {
        (self.midturn_active)()
    }

    pub async fn set_thinking_level(&mut self, level: String) -> Result<(), XyError> {
        let previous = self.thinking_level();
        self.with_models_mut(|mm| mm.set_thinking_level(level.clone()))?;
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

    pub fn apply_default_thinking_level(&mut self, raw: Option<&str>) {
        let preferred = raw.map(str::to_owned);
        self.with_models_mut(|mm| {
            mm.set_preferred_default(preferred);
            mm.apply_preferred_or_last();
        });
    }

    pub fn set_thinking_budgets(
        &mut self,
        budgets: Option<xylitol_plugin_sdk::model::ThinkingBudgets>,
    ) {
        self.with_models_mut(|mm| mm.set_thinking_budgets(budgets));
    }

    pub(crate) fn restore_thinking_level(&mut self, level: String) {
        self.with_models_mut(|mm| mm.restore_thinking_level(level));
    }

    pub async fn select_model(&mut self, model_id: &str) -> Result<(), XyError> {
        self.select_model_with_source(model_id, "set").await
    }

    pub async fn select_model_with_source(
        &mut self,
        model_id: &str,
        source: &str,
    ) -> Result<(), XyError> {
        let previous = self.current_model().map(|m| m.id.clone());
        self.with_models_mut(|mm| mm.select_model(model_id))?;
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
