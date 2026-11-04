use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;

use crate::agent::compaction::CompactionSettings;
use crate::agent::stage_name::{
    extract_stage_name_inputs, stage_name_input_json, stage_name_system_prompt,
};
use xylitol_plugin_sdk::error::{XyError, XySessionError};
use xylitol_plugin_sdk::message::AgentMessage;
use xylitol_plugin_sdk::model::{THINKING_OFF, XyChunk};
use xylitol_plugin_sdk::session::SessionEntry;
use xylitol_plugin_sdk::{
    XyGenerateOptions, XyLlmMessage, XySessionStore, fallback_stage_name, normalize_stage_name,
};

use super::{AgentCapabilities, SessionStats, observe_hook};

pub async fn load_conversation_history_from_store(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<Vec<AgentMessage>, XyError> {
    let entries = load_branch_with_migration(store, session_id).await?;
    let entries = xylitol_plugin_sdk::session::build_context_entries(&entries);
    let messages: Vec<AgentMessage> = entries
        .iter()
        .filter_map(|entry| entry.as_agent_message())
        .collect();
    Ok(fold_interrupted_bash(store, session_id, messages).await)
}

async fn load_branch_with_migration(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<Vec<SessionEntry>, XyError> {
    let mut entries = store
        .load_leaf_branch(session_id)
        .await
        .map_err(XyError::from)?;
    if xylitol_plugin_sdk::session::has_unmigrated_legacy_custom_types(&entries) {
        migrate_legacy_on_load(store, session_id).await;
        entries = store
            .load_leaf_branch(session_id)
            .await
            .map_err(XyError::from)?;
    }
    Ok(entries)
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
        if let Ok(Some(name)) = self.store.get_session_name(parent_id).await {
            self.store
                .set_session_name(&child_id, &name)
                .await
                .map_err(XyError::from)?;
        }

        Ok(child_id)
    }

    pub async fn get_session_stats(&self) -> Result<SessionStats, XyError> {
        let sid = self
            .session_id()
            .ok_or(XyError::from(XySessionError::NoActiveSession))?;
        crate::agent::capabilities::stats::compute(self.store.as_ref(), sid).await
    }

    pub async fn generate_session_name(&mut self, force: bool) -> Result<String, XyError> {
        let sid = self
            .session_id()
            .ok_or(XyError::from(XySessionError::NoActiveSession))?
            .to_string();
        let existing = self
            .store
            .get_session_name(&sid)
            .await
            .map_err(XyError::from)?;
        if !force && let Some(name) = existing.clone() {
            return Ok(name);
        }

        let entries = self
            .store
            .load_leaf_branch(&sid)
            .await
            .map_err(XyError::from)?;
        let (user, assistant) = extract_stage_name_inputs(&entries);
        let fallback = fallback_stage_name(
            &user,
            self.stage_name_config.target_words,
            self.stage_name_config.target_cjk_chars,
            self.stage_name_config.max_title_bytes,
        );

        if !self.stage_name_config.enabled || user.is_empty() {
            let name = existing.unwrap_or(fallback);
            self.persist_stage_name(&sid, &name).await?;
            return Ok(name);
        }

        let generated = self.generate_stage_name_with_model(&user, &assistant).await;
        let name = match generated {
            Ok(raw) => {
                let normalized = normalize_stage_name(&raw, self.stage_name_config.max_title_bytes);
                if normalized.is_empty() {
                    existing.unwrap_or(fallback)
                } else {
                    normalized
                }
            }
            Err(error) => {
                log::warn!(
                    target: "xylitol::session",
                    "stage name generation skipped session_id={sid} error={error}"
                );
                existing.unwrap_or(fallback)
            }
        };
        self.persist_stage_name(&sid, &name).await?;
        Ok(name)
    }

    async fn persist_stage_name(&mut self, session_id: &str, name: &str) -> Result<(), XyError> {
        let stored = self
            .store
            .set_session_name(session_id, name)
            .await
            .map_err(XyError::from)?;
        self.set_session_name_fact(Some(stored.clone()));
        xylitol_plugin_sdk::set_obs_session_name(Some(stored.as_str()));
        Ok(())
    }

    async fn generate_stage_name_with_model(
        &self,
        user: &str,
        assistant: &str,
    ) -> Result<String, XyError> {
        let (model, levels) = {
            let manager = self
                .model_manager
                .lock()
                .map_err(|_| XyError::Config("model manager lock poisoned".into()))?;
            manager.build_model_with_levels(self.stage_name_config.model.as_deref())?
        };
        let thinking_level = if levels.iter().any(|level| level == THINKING_OFF) {
            THINKING_OFF.to_string()
        } else {
            levels
                .first()
                .cloned()
                .unwrap_or_else(|| THINKING_OFF.to_string())
        };
        let input = stage_name_input_json(user, assistant, self.stage_name_config.max_input_bytes);
        let options = XyGenerateOptions {
            thinking_level,
            max_output_tokens: Some(self.stage_name_config.max_output_tokens),
            system_prompt: Some(stage_name_system_prompt().to_string()),
            obs_session: self.obs_session_snapshot(),
            ..Default::default()
        };
        let stream = model
            .generate_stream(vec![XyLlmMessage::user(input)], &[], false, options)
            .await?;
        let generated = tokio::time::timeout(
            Duration::from_millis(self.stage_name_config.timeout_ms),
            collect_stage_name_text(stream),
        )
        .await
        .map_err(|_| XyError::Config("stage name generation timed out".into()))??;
        Ok(generated)
    }
}

async fn collect_stage_name_text(
    mut stream: xylitol_plugin_sdk::XyStream,
) -> Result<String, XyError> {
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk? {
            XyChunk::TextDelta(delta) => text.push_str(&delta),
            XyChunk::Done { .. } => break,
            _ => {}
        }
    }
    Ok(text)
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

#[cfg(test)]
mod stage_name_tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use xylitol_plugin_sdk::message::{AgentMessage, XyStopReason};
    use xylitol_plugin_sdk::model::{XyChunk, XyToolSchema};
    use xylitol_plugin_sdk::session::{EntryBase, ForkPosition, MessageEntry, SessionEntry};
    use xylitol_plugin_sdk::{
        XyError, XyGenerateOptions, XyLlmMessage, XyModel, XyModelBuilder, XyModelConfig,
        XyModelKind, XyModelMeta, XyStream,
    };

    use crate::agent::capabilities::{AgentCapabilities, ModelRegistry, QueueMode};
    use crate::agent::stage_name::StageNameConfig;
    use crate::permission::allow_all_permission;
    use crate::test_support::{MemoryStore, NoopSink, fixture_tools};

    struct StaticModel;

    #[async_trait]
    impl XyModel for StaticModel {
        fn name(&self) -> &str {
            "stage-name-static"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<XyLlmMessage>,
            _tools: &[XyToolSchema],
            _stream: bool,
            _options: XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            Ok(Box::pin(futures::stream::iter(vec![
                Ok(XyChunk::TextDelta("新功能A".into())),
                Ok(XyChunk::Done {
                    finish_reason: XyStopReason::Stop,
                    usage: None,
                }),
            ])))
        }
    }

    fn meta(id: &str) -> XyModelMeta {
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
            thinking: false,
            context_window: 0,
            api: String::new(),
            provider: "fake".into(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
            thinking_level_map: Default::default(),
        }
    }

    fn message_entry(id: &str, message: AgentMessage) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 1,
            },
            message: serde_json::to_value(message).expect("message"),
        })
    }

    fn builder() -> XyModelBuilder {
        Arc::new(|_| Arc::new(StaticModel) as Arc<dyn XyModel>)
    }

    #[tokio::test]
    async fn generates_and_persists_stage_name() {
        let mut registry = ModelRegistry::new();
        registry.register(meta("fake-model"));
        let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> = Arc::new(MemoryStore::new());
        let sink: Arc<dyn xylitol_plugin_sdk::XyEventSink> = Arc::new(NoopSink);
        let mut caps = AgentCapabilities::new(
            registry,
            fixture_tools(),
            Arc::clone(&store),
            sink,
            Some("system".into()),
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            builder(),
            allow_all_permission(),
            QueueMode::default(),
            QueueMode::default(),
            None,
        );
        caps.select_model("fake-model").await.expect("select model");
        caps.set_session("s".into());
        caps.set_stage_name_config(StageNameConfig::default());
        store.create("s", Some("."), None).await.expect("create");
        store
            .append_session_entry(
                "s",
                &message_entry("u1", AgentMessage::user("帮我实现登录页")),
            )
            .await
            .expect("user");
        store
            .append_session_entry(
                "s",
                &message_entry("a1", AgentMessage::assistant("已完成登录页")),
            )
            .await
            .expect("assistant");

        let name = caps.generate_session_name(false).await.expect("generate");
        assert_eq!(name, "新功能A");
        assert_eq!(
            store.get_session_name("s").await.expect("name"),
            Some("新功能A".into())
        );
    }

    #[tokio::test]
    async fn fork_inherits_stage_name() {
        let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> = Arc::new(MemoryStore::new());
        let sink: Arc<dyn xylitol_plugin_sdk::XyEventSink> = Arc::new(NoopSink);
        let mut caps = AgentCapabilities::new(
            ModelRegistry::new(),
            fixture_tools(),
            Arc::clone(&store),
            sink,
            None,
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            builder(),
            allow_all_permission(),
            QueueMode::default(),
            QueueMode::default(),
            None,
        );
        caps.set_session("parent".into());
        store
            .create("parent", Some("."), None)
            .await
            .expect("create");
        store
            .append_session_entry("parent", &message_entry("u1", AgentMessage::user("父会话")))
            .await
            .expect("user");
        store
            .set_session_name("parent", "父阶段名")
            .await
            .expect("set name");

        let child = caps
            .fork_session("u1", ForkPosition::At)
            .await
            .expect("fork");
        assert_eq!(
            store.get_session_name(&child).await.expect("child name"),
            Some("父阶段名".into())
        );
    }

    #[tokio::test]
    async fn existing_name_is_pinned_without_force() {
        let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> = Arc::new(MemoryStore::new());
        let sink: Arc<dyn xylitol_plugin_sdk::XyEventSink> = Arc::new(NoopSink);
        let mut caps = AgentCapabilities::new(
            ModelRegistry::new(),
            fixture_tools(),
            Arc::clone(&store),
            sink,
            None,
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            builder(),
            allow_all_permission(),
            QueueMode::default(),
            QueueMode::default(),
            None,
        );
        caps.set_session("s".into());
        store.create("s", Some("."), None).await.expect("create");
        store
            .set_session_name("s", "已钉住")
            .await
            .expect("set name");

        let name = caps.generate_session_name(false).await.expect("generate");
        assert_eq!(name, "已钉住");
    }
}
