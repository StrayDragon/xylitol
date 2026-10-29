use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::error::{XyError, XySessionError};
use xylitol_plugin_sdk::message::AgentMessage;
use xylitol_plugin_sdk::session::SessionEntry;

use super::{AgentCapabilities, SessionStats, observe_hook};

pub async fn load_conversation_history_from_store(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<Vec<AgentMessage>, XyError> {
    let mut entries = store
        .load_leaf_branch(session_id)
        .await
        .map_err(XyError::from)?;
    migrate_legacy_if_needed(store, session_id, &mut entries).await?;
    let entries = xylitol_plugin_sdk::session::build_context_entries(&entries);
    let messages: Vec<AgentMessage> = entries
        .iter()
        .filter_map(|entry| entry.as_agent_message())
        .collect();
    Ok(fold_interrupted_bash(store, session_id, messages).await)
}

async fn migrate_legacy_if_needed(
    store: &dyn XySessionStore,
    session_id: &str,
    entries: &mut Vec<SessionEntry>,
) -> Result<(), XyError> {
    if xylitol_plugin_sdk::session::has_unmigrated_legacy_custom_types(entries) {
        migrate_legacy_on_load(store, session_id).await;
        *entries = store
            .load_leaf_branch(session_id)
            .await
            .map_err(XyError::from)?;
    }
    Ok(())
}

async fn migrate_legacy_on_load(store: &dyn XySessionStore, session_id: &str) {
    if let Err(error) = store.migrate_legacy_custom_types(session_id).await {
        log::warn!(
            target: "xylitol::session",
            "legacy custom-type migration skipped: {error}"
        );
    }
}

async fn fold_interrupted_bash(
    store: &dyn XySessionStore,
    session_id: &str,
    messages: Vec<AgentMessage>,
) -> Vec<AgentMessage> {
    match store.load_done_bash_ids(session_id).await {
        Ok(done) => xylitol_plugin_sdk::session::fold_interrupted_bash_rows(messages, &done),
        Err(error) => {
            log::warn!(
                target: "xylitol::session",
                "interrupted-bash fold skipped: load_done_bash_ids failed error={error}"
            );
            messages
        }
    }
}

impl AgentCapabilities {
    pub fn set_session(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    pub(crate) fn obs_session_snapshot(&self) -> xylitol_plugin_sdk::ObsSessionContext {
        xylitol_plugin_sdk::ObsSessionContext {
            session_id: self.session_id.clone(),
            session_name: self.session_name.clone(),
            ..Default::default()
        }
    }

    pub(crate) fn set_session_name_fact(&mut self, name: Option<String>) {
        self.session_name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    }

    pub(crate) fn session_name(&self) -> Option<&str> {
        self.session_name.as_deref()
    }

    pub(crate) async fn obs_session_snapshot_from_store(
        &self,
    ) -> xylitol_plugin_sdk::ObsSessionContext {
        let mut ctx = self.obs_session_snapshot();
        let Some(sid) = ctx.session_id.clone() else {
            return ctx;
        };
        if let Ok(entries) = self.store.load_entries(&sid).await {
            let (parent, cut) = xylitol_plugin_sdk::session::session_fork_edge(&entries);
            ctx.parent_session_id = parent;
            ctx.fork_at_entry_id = cut;
        }
        ctx
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub async fn ensure_session(&self, id: &str, parent: Option<&str>) -> Result<(), XyError> {
        let had_header = match self.store.load_entries(id).await {
            Ok(entries) => entries
                .iter()
                .any(|e| matches!(e, xylitol_plugin_sdk::session::SessionEntry::Header(_))),
            Err(_) => false,
        };
        let cwd_clone = self.cwd.clone();
        self.store
            .create(id, Some(&cwd_clone), parent)
            .await
            .map_err(XyError::from)?;
        if !had_header && let Some(bus) = &self.hook_bus {
            let reason = if parent.is_some() { "fork" } else { "new" };
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_start(reason);
            observe_hook(bus, ty, phase, ctx).await;
        }
        Ok(())
    }

    pub async fn load_conversation_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentMessage>, XyError> {
        load_conversation_history_from_store(self.store.as_ref(), session_id).await
    }

    pub fn session_store(&self) -> Arc<dyn XySessionStore> {
        self.store.clone()
    }

    pub(crate) fn event_sink(&self) -> Arc<dyn xylitol_plugin_sdk::XyEventSink> {
        self.sink.clone()
    }

    pub(crate) fn compaction_settings(&self) -> CompactionSettings {
        self.compaction_orchestrator.settings().clone()
    }

    pub(crate) fn compaction(&self) -> Arc<dyn crate::agent::compaction::XyCompaction> {
        Arc::clone(&self.compaction_orchestrator)
    }

    pub async fn fork_session(
        &self,
        at_entry_id: &str,
        position: xylitol_plugin_sdk::session::ForkPosition,
    ) -> Result<String, XyError> {
        let parent_id = self
            .session_id()
            .ok_or(XyError::from(XySessionError::NoActiveSession))?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_fork(
                at_entry_id,
                &format!("{position:?}"),
            );
            observe_hook(bus, ty, phase, ctx).await;
        }

        let child_id = uuid::Uuid::new_v4().to_string();

        self.store
            .fork(parent_id, &child_id, at_entry_id, position)
            .await
            .map_err(XyError::from)?;

        Ok(child_id)
    }

    pub async fn get_session_stats(&self) -> Result<SessionStats, XyError> {
        let sid = self
            .session_id()
            .ok_or(XyError::from(XySessionError::NoActiveSession))?;
        crate::agent::capabilities::stats::compute(self.store.as_ref(), sid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MemoryStore;
    use serde_json::json;
    use xylitol_plugin_sdk::session::{CustomEntry, EntryBase, SessionEntry};

    fn custom(custom_type: &str, data: serde_json::Value) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: String::new(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: custom_type.into(),
            data,
        })
    }

    #[tokio::test]
    async fn non_legacy_history_reads_branch_once() {
        let store = MemoryStore::new();
        store.create("s", None, None).await.expect("create");
        store
            .append_session_entry("s", &custom("workflow", json!({"goal": "x"})))
            .await
            .expect("append");
        load_conversation_history_from_store(&store, "s")
            .await
            .expect("load");
        assert_eq!(
            store.load_leaf_branch_calls(),
            1,
            "non-legacy history must not scan the branch twice"
        );
    }

    #[tokio::test]
    async fn legacy_history_migrates_once_then_reads_once() {
        let store = MemoryStore::new();
        store.create("s", None, None).await.expect("create");
        store
            .append_session_entry("s", &custom("plan", json!({"goal": "x"})))
            .await
            .expect("append");
        load_conversation_history_from_store(&store, "s")
            .await
            .expect("load");
        assert_eq!(
            store.load_leaf_branch_calls(),
            3,
            "legacy path must detect, migrate, and refresh"
        );
        let entries = store.load_entries("s").await.expect("entries");
        assert!(entries.iter().any(|entry| matches!(
            entry,
            SessionEntry::Custom(custom)
                if custom.custom_type == "workflow" && custom.data == json!({"goal": "x"})
        )));

        let before = store.load_leaf_branch_calls();
        load_conversation_history_from_store(&store, "s")
            .await
            .expect("load again");
        assert_eq!(
            store.load_leaf_branch_calls() - before,
            1,
            "after migration the hot path must read once"
        );
    }
}
