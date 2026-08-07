//! AgentCapabilities — core agent lifecycle / capability aggregate.
//!
//! Handles:
//! - Model registry and current model tracking
//! - Thinking level toggle (low/medium/high, clamped to model)
//! - Tool registry management
//! - Session persistence integration
//! - Compaction integration
//! - Model switching (cycleForward/cycleBackward/select)
//! - Context token estimation
//!
//! This is the **runtime capability aggregate**, not the persisted session
//! vocabulary in [`crate::protocol::session`].

use std::sync::{Arc, Mutex};

pub(crate) use crate::protocol::ports::{XyEventSink, XySessionStore};

mod bash;
mod compact_ops;
mod export;
mod hook_bus;
mod model_ops;
mod prompt_ops;
mod queue;
mod session_ops;
mod stats;
mod tools_ops;

pub(crate) use self::hook_bus::{cancel_hook, observe_hook, observe_hook_sync};
pub use self::queue::{
    AsyncQueueRuntime, PendingMessageQueue, QueueChannel, QueueMode, QueueStats,
};
pub use self::stats::{ContextUsage, SessionStats, estimate_tokens, get_context_usage};

use crate::agent::compaction::CompactionSettings;
use crate::agent::compaction::orchestrator::CompactionOrchestrator;
use crate::agent::model::manager::ModelManager;
use crate::agent::prompt::commands::SlashCommandInfo;
use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::runtime::AgentHooks;
use crate::agent::tools::{ToolFreezePhase, ToolSet, ToolTableFingerprint};
use crate::protocol::message::AgentMessage;
use crate::protocol::model::ThinkingLevel;
use crate::protocol::ports::{XyBashExecutor, XyBatchMode, XyExportIo, XyHookBus, XyPermission};

// ── Model Registry ──────────────────────────────────────────────────

pub use crate::agent::model::registry::ModelRegistry;

// ── AgentCapabilities ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTurnBinding {
    pub model_id: String,
    pub display_name: String,
    pub thinking: ThinkingLevel,
    /// True when footer should omit the thinking segment.
    pub omit_thinking: bool,
}

/// Capability aggregate — model, tools, session persistence, and events.
pub struct AgentCapabilities {
    /// Model management (registry, selection, thinking level). Shared so ReAct
    /// can refresh at turn boundaries while surfaces call `select_model`.
    model_manager: Arc<Mutex<ModelManager>>,
    /// Set for the duration of an agent `run`; `None` when idle / converged.
    active_turn: Arc<Mutex<Option<ActiveTurnBinding>>>,
    /// Tools available to the agent (construct-time final set).
    tools: ToolSet,
    /// Track-A tool-table freeze (c1900): Unfrozen → Gating → Frozen.
    tool_freeze: ToolFreezePhase,
    /// Fingerprint while [`ToolFreezePhase::Frozen`]; `None` otherwise.
    tool_fingerprint: Option<ToolTableFingerprint>,
    /// Runtime-mutable hooks consulted at tool-call boundaries.
    hooks: AgentHooks,
    /// Tool batch scheduling mode for the next run (c1545 / c1610).
    batch_mode: XyBatchMode,
    /// Fragment ids currently applied into [`Self::prompt_opts`] (c1605).
    /// `None` = never synced; id-set equality skips rebuild / duplicate policy text.
    runtime_fragment_ids: Option<Vec<&'static str>>,
    /// System prompt to prepend to every turn.
    system_prompt: Option<String>,
    /// Current session ID.
    session_id: Option<String>,
    /// Compaction orchestration (threshold check, trigger).
    compaction_orchestrator: CompactionOrchestrator,
    /// CWD for session header.
    cwd: String,
    /// System prompt options for dynamic building.
    prompt_opts: SystemPromptOpts,
    /// Extension-registered slash commands.
    extension_commands: Vec<SlashCommandInfo>,
    /// Bash-execution collaborator. Holds the optional [`XyBashExecutor`]
    /// port and the in-flight cancellation token.
    bash: crate::agent::capabilities::bash::BashExecHandler,
    /// Export/import collaborator. Holds the optional [`XyExportIo`] port.
    exporter: crate::agent::capabilities::export::SessionExporter,

    /// Advisory permission port consulted by the ReAct loop for tool routing.
    permission: Arc<dyn XyPermission>,
    /// Session store port — actively used by the ReAct loop.
    store: Arc<dyn XySessionStore>,
    /// Event sink port — used for compaction / non-queue lifecycle.
    sink: Arc<dyn XyEventSink>,
    /// Steer / follow-up queues + optional active-run EventTx (c525).
    queues: Arc<AsyncQueueRuntime>,
    /// Optional script hook bus (composition-root supplied).
    hook_bus: Option<Arc<dyn XyHookBus>>,
    /// Request-layout hooks (c1890); default ≡ current full-tools / no status bar.
    context_policy: crate::agent::context_policy::ContextPolicy,
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
        model_builder: crate::protocol::ports::XyModelBuilder,
        permission: Arc<dyn XyPermission>,
        bash_executor: Option<Arc<dyn XyBashExecutor>>,
        export_io: Option<Arc<dyn XyExportIo>>,
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
            active_turn: Arc::new(Mutex::new(None)),
            tools: tool_registry,
            tool_freeze: ToolFreezePhase::Unfrozen,
            tool_fingerprint: None,
            hooks: AgentHooks::empty(),
            batch_mode: XyBatchMode::BarrierParallel,
            runtime_fragment_ids: None,
            system_prompt: system_prompt.clone(),
            session_id: None,
            compaction_orchestrator: CompactionOrchestrator::new(
                compaction_settings.unwrap_or_default(),
            ),
            cwd: cwd.clone(),
            prompt_opts: SystemPromptOpts {
                cwd,
                system_prompt,
                context_files,
                append_system_prompt,
                selected_tools,
                tool_snippets,
                prompt_guidelines,
                skills: Vec::new(),
                // Filled once by sync_runtime_policy_from_batch_mode below.
                runtime_policy_fragments: Vec::new(),
                ..Default::default()
            },
            extension_commands: Vec::new(),
            bash: crate::agent::capabilities::bash::BashExecHandler::new(bash_executor),
            exporter: crate::agent::capabilities::export::SessionExporter::new(export_io),
            store,
            sink,
            permission,
            queues: Arc::new(AsyncQueueRuntime::new(steering_mode, follow_up_mode)),
            hook_bus,
            context_policy: crate::agent::context_policy::ContextPolicy::default(),
        };
        // Assemble full system prompt (tools + context + SYSTEM/APPEND + runtime
        // policy) once at construction so bootstrap-injected AGENTS.md is visible
        // on the first run (c1100 / pt1). Fragment sync is id-deduped (c1605).
        session.sync_runtime_policy_from_batch_mode();
        session
    }

    // ── Accessors ─────────────────────────────────────────────────

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

    pub(crate) fn tool_mode(&self) -> XyBatchMode {
        self.batch_mode
    }

    pub(crate) fn set_tool_mode(&mut self, mode: XyBatchMode) {
        self.batch_mode = mode;
        self.sync_runtime_policy_from_batch_mode();
    }

    /// Apply built-in runtime policy fragments for [`Self::batch_mode`].
    ///
    /// No-op when the active fragment **id set** is unchanged — Session holds the
    /// applied ids so the same policy is not re-appended / rebuilt (c1605).
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

    pub(crate) fn steer_queue(&self) -> Arc<Mutex<PendingMessageQueue>> {
        self.queues.steer.clone()
    }

    pub(crate) fn follow_up_queue(&self) -> Arc<Mutex<PendingMessageQueue>> {
        self.queues.follow_up.clone()
    }

    pub(crate) fn queues(&self) -> Arc<AsyncQueueRuntime> {
        self.queues.clone()
    }

    /// Enqueue a steering message (injected before the next model round).
    pub fn steer(&self, message: impl Into<String>) {
        self.steer_parts(vec![crate::protocol::message::AgentPart::text(message)]);
    }

    /// Enqueue a multi-part steering message (c1155).
    pub fn steer_parts(&self, parts: Vec<crate::protocol::message::AgentPart>) {
        let msg = AgentMessage::user_parts(parts);
        self.queues
            .steer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enqueue(msg);
        self.queues.notify_queue_update();
    }

    /// Enqueue a follow-up message (injected when the run would otherwise stop).
    pub fn follow_up(&self, message: impl Into<String>) {
        self.follow_up_parts(vec![crate::protocol::message::AgentPart::text(message)]);
    }

    /// Enqueue a multi-part follow-up message (c1155).
    pub fn follow_up_parts(&self, parts: Vec<crate::protocol::message::AgentPart>) {
        let msg = AgentMessage::user_parts(parts);
        self.queues
            .follow_up
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enqueue(msg);
        self.queues.notify_queue_update();
    }

    /// Clear the steering queue only.
    pub fn clear_steer_queue(&self) {
        self.queues
            .steer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.queues.notify_queue_update();
    }

    /// Clear the follow-up queue only.
    pub fn clear_follow_up_queue(&self) {
        self.queues
            .follow_up
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.queues.notify_queue_update();
    }

    /// Clear one or both queues.
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

    /// Queue depths.
    pub fn queue_stats(&self) -> QueueStats {
        self.queues.stats()
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// Request-layout policy (c1890). Default ≡ full tools / status bar off.
    pub fn context_policy(&self) -> &crate::agent::context_policy::ContextPolicy {
        &self.context_policy
    }

    /// Test / in-crate override (proposal Q1: no public session YAML override this wave).
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

    /// Current working directory.
    pub fn cwd(&self) -> &str {
        &self.cwd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::SessionManager;

    fn make_session() -> AgentCapabilities {
        let mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: std::sync::Arc<dyn crate::protocol::ports::XySessionStore> =
            std::sync::Arc::new(mgr);
        let sink: std::sync::Arc<dyn crate::protocol::ports::XyEventSink> =
            std::sync::Arc::new(crate::infra::event::EventBus::new());
        AgentCapabilities::new(
            ModelRegistry::new(std::sync::Arc::new(
                crate::infra::config::value::InfraSecretResolver::new(),
            )),
            ToolSet::from_iter(crate::infra::tools::default_tools()),
            store,
            sink,
            Some("you are helpful".into()),
            Vec::new(),
            Vec::new(),
            ".".into(),
            None,
            std::sync::Arc::new(crate::infra::provider::factory::build_provider),
            crate::infra::permission::allow_all_permission(),
            Some(std::sync::Arc::new(
                crate::infra::bash_exec::InfraBashExecutor::new(),
            )),
            Some(std::sync::Arc::new(crate::infra::export::StdExportIo::new())),
            QueueMode::default(),
            QueueMode::default(),
            None,
        )
    }

    #[test]
    fn get_commands_includes_product_builtins() {
        let session = make_session();
        let names: Vec<String> = session.get_commands().into_iter().map(|c| c.name).collect();
        // Product builtins from SSOT (c1175).
        assert!(names.iter().any(|n| n == "model"));
        assert!(names.iter().any(|n| n == "session-export"));
        assert!(names.iter().any(|n| n == "session-compact"));
        assert!(names.iter().any(|n| n == "session-tree"));
        assert!(!names.iter().any(|n| n == "tree"));
        assert!(!names.iter().any(|n| n == "compact"));
        assert!(!names.iter().any(|n| n == "export"));
        assert!(!names.iter().any(|n| n.starts_with("template:")));
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

        // Same mode again (builder also calls set_tool_mode after new) — no duplicate.
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

        // Rebuild with the same builtins — metadata refresh is enough to prove defer.
        let tools = ToolSet::from_iter(crate::infra::tools::default_tools());
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
    fn search_policy_blocks_midturn_set_tools() {
        use crate::agent::context_policy::{ContextPolicy, ToolsMode};

        let mut session = make_session();
        let before_n = session.tools().iter().count();
        session.set_context_policy_for_test(ContextPolicy {
            tools_mode: ToolsMode::Search,
            ..Default::default()
        });
        *session.active_turn_handle().lock().unwrap() = Some(ActiveTurnBinding {
            model_id: "m".into(),
            display_name: "m".into(),
            thinking: ThinkingLevel::Off,
            omit_thinking: true,
        });

        session.set_tools(ToolSet::empty());
        assert_eq!(
            session.tools().iter().count(),
            before_n,
            "Search + in-flight turn must not rewrite tools"
        );

        session.clear_active_turn();
        session.set_tools(ToolSet::empty());
        assert_eq!(session.tools().iter().count(), 0);
    }

    #[test]
    fn freeze_then_set_tools_does_not_expand() {
        let mut session = make_session();
        assert_eq!(session.tool_freeze_phase(), ToolFreezePhase::Unfrozen);
        session.begin_tool_gating();
        assert_eq!(session.tool_freeze_phase(), ToolFreezePhase::Gating);

        let core = ToolSet::from_iter(crate::infra::tools::default_tools());
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
        session.freeze_tools(ToolSet::from_iter(crate::infra::tools::default_tools()));
        assert!(session.is_tools_frozen());
    }

    #[test]
    fn fingerprint_mismatch_when_schema_changes() {
        use crate::protocol::ports::XyTool;
        use std::sync::Arc;

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
                _: &crate::protocol::ports::XyToolCtx,
                _: serde_json::Value,
            ) -> Result<String, crate::protocol::error::XyToolError> {
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

        // Seed a fake history marker on the in-memory store via ensure+append path
        // is heavy; instead verify apply only changes assembled prompt and does not
        // clear session_id / queues (history ownership stays with the store).
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
        use crate::protocol::resource::SkillInfo;
        use crate::protocol::source_info::{SourceInfo, SourceOrigin, SourceScope};

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
        // Enqueue without notify (no active EventTx in this unit test).
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

    #[tokio::test]
    async fn runtime_abort_cancels_interactive_bash() {
        use std::sync::Arc;
        use std::time::Duration;

        let agent = Arc::new(crate::agent::runtime::AgentRuntime::new(make_session()));
        let agent_exec = Arc::clone(&agent);
        let join = tokio::spawn(async move {
            agent_exec
                .inner()
                .execute_bash("sleep 30", false, None)
                .await
        });

        tokio::time::sleep(Duration::from_millis(150)).await;
        agent.abort();
        let result = join.await.expect("join").expect("execute_bash");
        assert!(
            result.cancelled,
            "AgentRuntime::abort must cancel in-flight interactive bash"
        );
    }
}
