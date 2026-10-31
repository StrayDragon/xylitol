use std::sync::{Arc, Mutex};

pub(crate) use xylitol_plugin_sdk::{XyEventSink, XySessionStore};

mod compact_ops;
mod hook_bus;
mod model_ops;
mod prompt_ops;
mod queue;
mod session_ops;
mod stats;
mod tools_ops;

pub use self::hook_bus::HookBlockedError;
pub(crate) use self::hook_bus::{cancel_hook, observe_hook, observe_hook_sync};
pub use self::queue::{AsyncQueueRuntime, PendingMessageQueue, QueueMode, QueueStats};
pub(crate) use self::session_ops::load_conversation_history_from_store;
pub use self::stats::SessionStats;
pub use self::stats::{ContextUsage, get_context_usage};

use crate::agent::compaction::{
    CompactionSettings, XyCompaction, XyCompactionFactory, noop_compaction_factory,
};
use crate::agent::model::manager::ModelManager;
use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::runtime::AgentHooks;
use crate::agent::tools::{ToolFreezePhase, ToolSet, ToolTableFingerprint};
use xylitol_plugin_sdk::message::AgentMessage;
use xylitol_plugin_sdk::model::thinking_levels_are_adjustable;
use xylitol_plugin_sdk::{XyBatchMode, XyHookBus, XyPermission};

pub use crate::agent::model::registry::ModelRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTurnBinding {
    pub model_id: String,
    pub display_name: String,
    pub thinking: String,
    pub omit_thinking: bool,
}

impl ActiveTurnBinding {
    pub(crate) fn from_manager(mm: &ModelManager) -> Option<Self> {
        let meta = mm.current_model()?;
        let levels = ModelManager::levels_for_meta(meta);
        Some(Self {
            model_id: meta.id.clone(),
            display_name: if meta.display_name.is_empty() {
                meta.id.clone()
            } else {
                meta.display_name.clone()
            },
            thinking: mm.thinking_level(),
            omit_thinking: !thinking_levels_are_adjustable(&levels),
        })
    }
}

pub struct AgentCapabilities {
    model_manager: Arc<Mutex<ModelManager>>,
    midturn_active: Arc<dyn Fn() -> bool + Send + Sync>,
    tools: ToolSet,
    tool_freeze: ToolFreezePhase,
    tool_fingerprint: Option<ToolTableFingerprint>,
    hooks: AgentHooks,
    batch_mode: XyBatchMode,
    runtime_fragment_ids: Option<Vec<&'static str>>,
    system_prompt: Option<String>,
    session_id: Option<String>,
    compaction_orchestrator: Arc<dyn XyCompaction>,
    compaction_factory: XyCompactionFactory,
    cwd: String,
    prompt_opts: SystemPromptOpts,

    permission: Arc<dyn XyPermission>,
    store: Arc<dyn XySessionStore>,
    sink: Arc<dyn XyEventSink>,
    queues: Arc<AsyncQueueRuntime>,
    hook_bus: Option<Arc<dyn XyHookBus>>,
    context_policy: crate::agent::context_policy::ContextPolicy,
    session_name: Option<String>,
    loop_registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
    loop_scope: xylitol_plugin_sdk::LoopScope,
}

impl AgentCapabilities {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_registry: ModelRegistry,
        tool_registry: ToolSet,
        store: Arc<dyn XySessionStore>,
        sink: Arc<dyn XyEventSink>,
        system_prompt: Option<String>,
        context_files: Vec<(String, String)>,
        append_system_prompt: Vec<String>,
        cwd: String,
        compaction_settings: Option<CompactionSettings>,
        model_builder: xylitol_plugin_sdk::XyModelBuilder,
        permission: Arc<dyn XyPermission>,
        steering_mode: QueueMode,
        follow_up_mode: QueueMode,
        hook_bus: Option<Arc<dyn XyHookBus>>,
    ) -> Self {
        let selected_tools: Vec<String> =
            tool_registry.iter().map(|t| t.name().to_string()).collect();
        let tool_snippets = prompt::collect_tool_snippets(&tool_registry, &selected_tools);
        let prompt_guidelines = prompt::collect_tool_guidelines(&tool_registry, &selected_tools);

        let mut session = Self {
            model_manager: Arc::new(Mutex::new(ModelManager::new(model_registry, model_builder))),
            midturn_active: Arc::new(|| false),
            tools: tool_registry,
            tool_freeze: ToolFreezePhase::Unfrozen,
            tool_fingerprint: None,
            hooks: AgentHooks::empty(),
            batch_mode: XyBatchMode::BarrierParallel,
            runtime_fragment_ids: None,
            system_prompt: system_prompt.clone(),
            session_id: None,
            compaction_orchestrator: noop_compaction_factory()(
                compaction_settings.unwrap_or_default(),
            ),
            compaction_factory: noop_compaction_factory(),
            cwd: cwd.clone(),
            prompt_opts: SystemPromptOpts {
                system_prompt,
                context_files,
                append_system_prompt,
                selected_tools,
                tool_snippets,
                prompt_guidelines,
                skills: Vec::new(),
                runtime_policy_fragments: Vec::new(),
                ..Default::default()
            },
            store,
            sink,
            permission,
            queues: Arc::new(AsyncQueueRuntime::new(steering_mode, follow_up_mode)),
            hook_bus,
            context_policy: crate::agent::context_policy::ContextPolicy::default(),
            session_name: None,
            loop_registry: None,
            loop_scope: xylitol_plugin_sdk::LoopScope::Root,
        };
        session.sync_runtime_policy_from_batch_mode();
        session
    }

    pub fn install_compaction(&mut self, factory: XyCompactionFactory) {
        let settings = self.compaction_orchestrator.settings().clone();
        self.compaction_factory = factory.clone();
        self.compaction_orchestrator = factory(settings);
    }

    pub(crate) fn tools(&self) -> &ToolSet {
        &self.tools
    }

    pub(crate) fn hooks(&self) -> &AgentHooks {
        &self.hooks
    }

    pub(crate) fn hooks_mut(&mut self) -> &mut AgentHooks {
        &mut self.hooks
    }

    pub(crate) fn hook_bus(&self) -> Option<Arc<dyn XyHookBus>> {
        self.hook_bus.clone()
    }

    pub(crate) fn set_loop_registry(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
    ) {
        self.loop_registry = registry;
    }

    pub(crate) fn loop_registry(&self) -> Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>> {
        self.loop_registry.clone()
    }

    pub(crate) fn set_loop_scope(&mut self, scope: xylitol_plugin_sdk::LoopScope) {
        self.loop_scope = scope;
    }

    pub(crate) fn loop_chain(&self) -> xylitol_plugin_sdk::LoopChain {
        let mut chain = self
            .loop_registry
            .as_ref()
            .map(|registry| registry.snapshot())
            .unwrap_or_default();
        chain.scope = self.loop_scope;
        chain
    }

    pub(crate) fn tool_mode(&self) -> XyBatchMode {
        self.batch_mode
    }

    pub(crate) fn set_tool_mode(&mut self, mode: XyBatchMode) {
        self.batch_mode = mode;
        self.sync_runtime_policy_from_batch_mode();
    }

    fn sync_runtime_policy_from_batch_mode(&mut self) {
        let ids = prompt::fragment_ids_for_batch_mode(self.batch_mode);
        if self.runtime_fragment_ids.as_deref() == Some(ids.as_slice()) {
            return;
        }
        self.runtime_fragment_ids = Some(ids);
        let mut bodies: Vec<String> = prompt::fragments_for_batch_mode(self.batch_mode)
            .into_iter()
            .map(str::to_string)
            .collect();
        let mut seen = std::collections::HashSet::new();
        bodies.retain(|b| seen.insert(b.clone()));
        self.prompt_opts.runtime_policy_fragments = bodies;
        self.rebuild_system_prompt();
    }

    pub(crate) fn queues(&self) -> Arc<AsyncQueueRuntime> {
        self.queues.clone()
    }

    pub fn steer(&self, message: impl Into<String>) {
        self.steer_parts(vec![xylitol_plugin_sdk::message::AgentPart::text(message)]);
    }

    pub fn steer_parts(&self, parts: Vec<xylitol_plugin_sdk::message::AgentPart>) {
        let msg = AgentMessage::user_parts(parts);
        self.queues
            .steer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enqueue(msg);
        self.queues.notify_queue_update();
    }

    pub fn follow_up(&self, message: impl Into<String>) {
        self.follow_up_parts(vec![xylitol_plugin_sdk::message::AgentPart::text(message)]);
    }

    pub fn follow_up_parts(&self, parts: Vec<xylitol_plugin_sdk::message::AgentPart>) {
        let msg = AgentMessage::user_parts(parts);
        self.queues
            .follow_up
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enqueue(msg);
        self.queues.notify_queue_update();
    }

    pub fn clear_steer_queue(&self) {
        self.queues
            .steer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.queues.notify_queue_update();
    }

    pub fn clear_queues(&self, clear_steer: bool, clear_follow_up: bool) {
        if clear_steer {
            self.queues
                .steer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
        }
        if clear_follow_up {
            self.queues
                .follow_up
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
        }
        if clear_steer || clear_follow_up {
            self.queues.notify_queue_update();
        }
    }

    pub fn queue_stats(&self) -> QueueStats {
        self.queues.stats()
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn set_context_policy_for_test(
        &mut self,
        policy: crate::agent::context_policy::ContextPolicy,
    ) {
        self.context_policy = policy;
    }

    pub fn model_registry(&self) -> ModelRegistry {
        self.with_models(|mm| mm.registry().clone())
    }

    pub fn replace_model_registry(&self, registry: ModelRegistry) {
        self.with_models_mut(|mm| mm.replace_registry(registry));
    }

    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    pub fn set_cwd(&mut self, cwd: impl Into<String>) {
        self.cwd = cwd.into();
    }

    pub(crate) fn set_midturn_active_probe(&mut self, probe: Arc<dyn Fn() -> bool + Send + Sync>) {
        self.midturn_active = probe;
    }

    #[cfg(test)]
    pub(crate) fn set_midturn_active_for_test(&mut self, active: bool) {
        self.midturn_active = Arc::new(move || active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MemoryStore, NoopSink, dummy_model_builder, fixture_tools};

    fn make_session() -> AgentCapabilities {
        let store: std::sync::Arc<dyn xylitol_plugin_sdk::XySessionStore> =
            std::sync::Arc::new(MemoryStore::new());
        let sink: std::sync::Arc<dyn xylitol_plugin_sdk::XyEventSink> =
            std::sync::Arc::new(NoopSink);
        AgentCapabilities::new(
            ModelRegistry::new(),
            fixture_tools(),
            store,
            sink,
            Some("you are helpful".into()),
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            dummy_model_builder(),
            crate::permission::allow_all_permission(),
            QueueMode::default(),
            QueueMode::default(),
            None,
        )
    }

    #[test]
    fn default_system_prompt_omits_session_env() {
        let session = make_session();
        let prompt = session.system_prompt().unwrap_or("").to_string();
        assert!(!prompt.contains("Current date:"), "{prompt}");
        assert!(!prompt.contains("Current working directory:"), "{prompt}");
    }

    #[test]
    fn runtime_policy_synced_once_per_fragment_id_set() {
        let mut session = make_session();
        let prompt = session.system_prompt().unwrap_or("").to_string();
        assert_eq!(prompt.matches("<runtime_policy>").count(), 1);
        assert_eq!(prompt.matches("SAME assistant message").count(), 1);
        assert_eq!(
            session.runtime_fragment_ids.as_deref(),
            Some(
                [crate::agent::prompt::fragments::FRAGMENT_TOOL_BATCH_BARRIER_PARALLEL].as_slice()
            )
        );

        session.set_tool_mode(XyBatchMode::BarrierParallel);
        let again = session.system_prompt().unwrap_or("").to_string();
        assert_eq!(again.matches("<runtime_policy>").count(), 1);
        assert_eq!(again.matches("SAME assistant message").count(), 1);

        session.set_tool_mode(XyBatchMode::Sequential);
        let sequential = session.system_prompt().unwrap_or("").to_string();
        assert!(!sequential.contains("<runtime_policy>"));
        assert_eq!(session.runtime_fragment_ids.as_deref(), Some([].as_slice()));

        session.set_tool_mode(XyBatchMode::BarrierParallel);
        let restored = session.system_prompt().unwrap_or("").to_string();
        assert_eq!(restored.matches("<runtime_policy>").count(), 1);
        assert_eq!(restored.matches("SAME assistant message").count(), 1);
    }

    #[test]
    fn set_tools_defer_prompt_keeps_stale_prompt_until_install() {
        let mut session = make_session();
        let before = session.system_prompt().unwrap_or("").to_string();
        assert!(!before.is_empty());

        let tools = fixture_tools();
        let opts = session.set_tools_defer_prompt(tools);
        assert!(
            session.tools().iter().any(|t| t.name() == "read"),
            "tools must be installed immediately"
        );
        assert_eq!(
            session.system_prompt().unwrap_or(""),
            before,
            "system prompt text must stay stale until install"
        );

        let built = crate::agent::prompt::build_system_prompt(&opts);
        session.install_system_prompt_text(built.clone());
        assert_eq!(session.system_prompt().unwrap_or(""), built);
        assert!(
            session.system_prompt().unwrap_or("").contains("read") || !built.is_empty(),
            "installed prompt should reflect tool set"
        );
    }

    #[test]
    fn rewrite_flag_blocks_midturn_set_tools() {
        use crate::agent::context_policy::ContextPolicy;

        let mut session = make_session();
        let before_n = session.tools().iter().count();
        session.set_context_policy_for_test(ContextPolicy {
            allow_midturn_tools_rewrite: false,
            ..Default::default()
        });
        session.set_midturn_active_for_test(true);

        session.set_tools(ToolSet::empty());
        assert_eq!(
            session.tools().iter().count(),
            before_n,
            "deny flag + in-flight turn must not rewrite tools"
        );

        session.set_midturn_active_for_test(false);
        session.set_tools(ToolSet::empty());
        assert_eq!(session.tools().iter().count(), 0);
    }

    #[test]
    fn freeze_includes_todo_builtins_first_turn() {
        let mut session = make_session();
        session.begin_tool_gating();
        let core = fixture_tools();
        session.freeze_tools(core);
        let fp = session.frozen_tool_fingerprint().cloned().expect("fp");
        for n in ["todo_rewrite", "todo_update"] {
            assert!(
                fp.names.iter().any(|x| x == n),
                "frozen table missing {n}: {:?}",
                fp.names
            );
        }
        session.set_tools(fixture_tools());
        assert_eq!(
            session.tools().iter().count(),
            fp.names.len(),
            "frozen set size must stay put"
        );
    }

    #[test]
    fn freeze_then_set_tools_does_not_expand() {
        let mut session = make_session();
        assert_eq!(session.tool_freeze_phase(), ToolFreezePhase::Unfrozen);
        session.begin_tool_gating();
        assert_eq!(session.tool_freeze_phase(), ToolFreezePhase::Gating);

        let core = fixture_tools();
        let n_core = core.iter().count();
        session.freeze_tools(core);
        assert!(session.is_tools_frozen());
        let fp = session.frozen_tool_fingerprint().cloned().expect("fp");
        assert_eq!(fp.names.len(), n_core);
        assert!(session.frozen_fingerprint_matches_set(session.tools()));

        let before = session.tools().iter().count();
        session.set_tools(ToolSet::empty());
        session.set_tools_defer_prompt(ToolSet::empty());
        assert_eq!(session.tools().iter().count(), before);
        assert!(session.is_tools_frozen());

        session.reopen_tools_for_regate();
        assert_eq!(session.tool_freeze_phase(), ToolFreezePhase::Gating);
        assert!(session.frozen_tool_fingerprint().is_none());
        session.freeze_tools(fixture_tools());
        assert!(session.is_tools_frozen());
    }

    #[test]
    fn fingerprint_mismatch_when_schema_changes() {
        use std::sync::Arc;
        use xylitol_plugin_sdk::XyTool;

        struct Named(&'static str, &'static str, serde_json::Value);
        #[async_trait::async_trait]
        impl XyTool for Named {
            fn name(&self) -> &str {
                self.0
            }
            fn description(&self) -> &str {
                self.1
            }
            fn parameters_schema(&self) -> serde_json::Value {
                self.2.clone()
            }
            async fn execute(
                &self,
                _: &xylitol_plugin_sdk::XyToolCtx,
                _: serde_json::Value,
            ) -> Result<String, xylitol_plugin_sdk::error::XyToolError> {
                Ok("ok".into())
            }
        }

        let mut session = make_session();
        let a =
            ToolSet::from_iter(vec![
                Arc::new(Named("t", "d", serde_json::json!({"type": "object"}))) as Arc<dyn XyTool>,
            ]);
        session.freeze_tools(a);
        let b = ToolSet::from_iter(vec![Arc::new(Named(
            "t",
            "d",
            serde_json::json!({"type": "object", "required": ["x"]}),
        )) as Arc<dyn XyTool>]);
        assert!(!session.frozen_fingerprint_matches_set(&b));
    }

    #[test]
    fn apply_prompt_resources_keeps_single_runtime_policy() {
        let mut session = make_session();
        session.apply_prompt_resources(
            vec![("AGENTS.md".into(), "CTX".into())],
            Some("system-base".into()),
            vec!["APPEND_MARK".into()],
        );
        let after = session.system_prompt().unwrap_or("").to_string();
        assert!(after.contains("APPEND_MARK"));
        assert_eq!(after.matches("<runtime_policy>").count(), 1);
        assert_eq!(after.matches("SAME assistant message").count(), 1);
    }

    #[test]
    fn apply_prompt_resources_updates_system_keeps_history_untouched() {
        let mut session = make_session();
        let before = session.system_prompt().unwrap_or("").to_string();
        assert!(!before.contains("UNIQUE_CONTEXT_MARKER_V2"));

        session
            .queues
            .steer
            .lock()
            .unwrap()
            .enqueue(AgentMessage::user("steer-keep"));
        let stats_before = session.queue_stats();

        session.apply_prompt_resources(
            vec![("AGENTS.md".into(), "UNIQUE_CONTEXT_MARKER_V2".into())],
            Some("system-base".into()),
            vec!["APPEND_MARK".into()],
        );

        let after = session.system_prompt().unwrap_or("").to_string();
        assert!(after.contains("UNIQUE_CONTEXT_MARKER_V2"));
        assert!(after.contains("APPEND_MARK"));
        assert_eq!(session.queue_stats(), stats_before);
    }

    #[test]
    fn apply_skills_updates_system_keeps_queues_untouched() {
        use xylitol_plugin_sdk::resource::SkillInfo;
        use xylitol_plugin_sdk::source_info::{SourceInfo, SourceOrigin, SourceScope};

        let mut session = make_session();
        session
            .queues
            .steer
            .lock()
            .unwrap()
            .enqueue(AgentMessage::user("steer-keep"));
        let stats_before = session.queue_stats();

        session.apply_skills(vec![SkillInfo {
            name: "demo-skill".into(),
            description: Some("demo".into()),
            source_info: SourceInfo {
                path: std::path::PathBuf::from("/tmp/skills/demo/SKILL.md"),
                source: "local".into(),
                scope: SourceScope::User,
                origin: SourceOrigin::TopLevel,
                base_dir: None,
            },
            disable_model_invocation: false,
        }]);

        let after = session.system_prompt().unwrap_or("").to_string();
        assert!(after.contains("<available_skills>"));
        assert!(after.contains("demo-skill"));
        assert_eq!(session.loaded_skill_names(), vec!["demo-skill".to_string()]);
        assert_eq!(session.queue_stats(), stats_before);
    }

    #[tokio::test]
    async fn abort_clears_steer_keeps_follow_up() {
        let session = make_session();
        session
            .queues
            .steer
            .lock()
            .unwrap()
            .enqueue(AgentMessage::user("steer-me"));
        session
            .queues
            .follow_up
            .lock()
            .unwrap()
            .enqueue(AgentMessage::user("follow-me"));
        assert_eq!(
            session.queue_stats(),
            QueueStats {
                steer_count: 1,
                follow_up_count: 1
            }
        );

        let agent = crate::agent::runtime::AgentRuntime::new(session);
        agent.abort();
        assert_eq!(
            agent.queue_stats(),
            QueueStats {
                steer_count: 0,
                follow_up_count: 1
            }
        );
    }

    fn model_meta(
        id: &str,
        thinking: bool,
        levels: &[&str],
    ) -> xylitol_plugin_sdk::model::XyModelMeta {
        xylitol_plugin_sdk::model::XyModelMeta {
            id: id.into(),
            config: xylitol_plugin_sdk::model::XyModelConfig {
                kind: xylitol_plugin_sdk::model::XyModelKind::Fake,
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

    fn make_session_with_models(
        models: Vec<xylitol_plugin_sdk::model::XyModelMeta>,
    ) -> AgentCapabilities {
        let store: std::sync::Arc<dyn XySessionStore> = std::sync::Arc::new(MemoryStore::new());
        let sink: std::sync::Arc<dyn XyEventSink> = std::sync::Arc::new(NoopSink);
        let mut registry = ModelRegistry::new();
        for m in models {
            registry.register(m);
        }
        AgentCapabilities::new(
            registry,
            fixture_tools(),
            store.clone(),
            sink,
            Some("you are helpful".into()),
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            dummy_model_builder(),
            crate::permission::allow_all_permission(),
            QueueMode::default(),
            QueueMode::default(),
            None,
        )
    }

    #[tokio::test]
    async fn select_model_same_id_does_not_persist_model_change() {
        use xylitol_plugin_sdk::session::SessionEntry;
        let sid = "cap-modelchange-guard";
        let mut session = make_session_with_models(vec![model_meta("m-a", true, &["off", "high"])]);
        session.set_session(sid.into());

        session.select_model("m-a").await.unwrap();
        session.select_model("m-a").await.unwrap();

        let entries = session.session_store().load_entries(sid).await.unwrap();
        let changes = entries
            .iter()
            .filter(|e| matches!(e, SessionEntry::ModelChange(_)))
            .count();
        assert_eq!(
            changes, 1,
            "expected exactly one modelChange, got {changes}"
        );
    }

    #[tokio::test]
    async fn restore_source_select_persists_nothing() {
        use xylitol_plugin_sdk::session::SessionEntry;
        let sid = "cap-modelchange-restore";
        let mut session = make_session_with_models(vec![model_meta("m-a", true, &["off", "high"])]);
        session.set_session(sid.into());
        session.ensure_session(sid, None).await.unwrap();

        session
            .select_model_with_source("m-a", "restore")
            .await
            .unwrap();

        let entries = session.session_store().load_entries(sid).await.unwrap();
        assert!(
            entries
                .iter()
                .all(|e| !matches!(e, SessionEntry::ModelChange(_))),
            "restore select MUST NOT persist modelChange, got {entries:?}"
        );
    }

    #[tokio::test]
    async fn set_thinking_level_same_value_does_not_persist_entry() {
        use xylitol_plugin_sdk::session::SessionEntry;
        let sid = "cap-thinking-guard";
        let mut session = make_session_with_models(vec![model_meta("m-a", true, &["off", "high"])]);
        session.set_session(sid.into());
        session.select_model("m-a").await.unwrap();

        session.set_thinking_level("off".into()).await.unwrap();
        session.set_thinking_level("off".into()).await.unwrap();

        let entries = session.session_store().load_entries(sid).await.unwrap();
        let changes = entries
            .iter()
            .filter(|e| matches!(e, SessionEntry::ThinkingLevelChange(_)))
            .count();
        assert_eq!(changes, 1, "expected exactly one thinkingLevelChange");
    }
}
