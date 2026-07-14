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

use std::sync::{Arc, Mutex};

pub(crate) use crate::runtime_protocol::{XyEventSink, XySessionStore};

mod bash;
mod export;
mod queue;
mod stats;
mod trust;

pub use self::queue::{
    AsyncQueueRuntime, PendingMessageQueue, QueueChannel, QueueMode, QueueStats,
};
pub use self::stats::{ContextUsage, SessionStats, estimate_tokens, get_context_usage};
pub use self::trust::save_trust_decision;

use crate::agent::compaction::CompactionSettings;
use crate::agent::compaction::orchestrator::CompactionOrchestrator;
use crate::agent::model::manager::ModelManager;
use crate::agent::prompt::commands::{SlashCommandInfo, get_all_commands};
use crate::agent::prompt::templates::PromptTemplate;
use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::runtime::AgentHooks;
use crate::agent::tools::ToolSet;
use crate::domain::message::AgentMessage;
use crate::domain::session_types::{
    EntryBase, ModelChangeEntry, SessionEntry, ThinkingLevelChangeEntry,
};
#[cfg(test)]
use crate::domain::source_info::{SourceInfo, SourceOrigin, SourceScope};
use crate::domain::types::{ThinkingLevel, XyModelMeta};
use crate::runtime_protocol::{
    XyBashExecutor, XyExportIo, XyModel, XyPermission, XyToolExecutionMode,
};

// ── Model Registry ──────────────────────────────────────────────────

pub use crate::agent::model::registry::ModelRegistry;

// ── AgentCapabilities ────────────────────────────────────────────────────

/// Capability aggregate — model, tools, session persistence, and events.
pub struct AgentCapabilities {
    /// Model management (registry, selection, thinking level).
    model_manager: ModelManager,
    /// Tools available to the agent (construct-time final set).
    tools: ToolSet,
    /// Runtime-mutable hooks consulted at tool-call boundaries.
    hooks: AgentHooks,
    /// Tool execution mode for the current turn.
    tool_mode: XyToolExecutionMode,
    /// System prompt to prepend to every turn.
    system_prompt: Option<String>,
    /// Current session ID.
    session_id: Option<String>,
    /// Max ReAct loop iterations per turn.
    max_iterations: u32,
    /// Compaction orchestration (threshold check, trigger).
    compaction_orchestrator: CompactionOrchestrator,
    /// CWD for session header.
    cwd: String,
    /// System prompt options for dynamic building.
    prompt_opts: SystemPromptOpts,
    /// Registered prompt templates for /template:name expansion.
    prompt_templates: Vec<PromptTemplate>,
    /// Extension-registered slash commands.
    extension_commands: Vec<SlashCommandInfo>,
    /// Bash-execution collaborator. Holds the optional [`XyBashExecutor`]
    /// port and the in-flight cancellation token.
    bash: crate::agent::session::bash::BashExecHandler,
    /// Export/import collaborator. Holds the optional [`XyExportIo`] port.
    exporter: crate::agent::session::export::SessionExporter,

    /// Advisory permission port consulted by the ReAct loop for tool routing.
    permission: Arc<dyn XyPermission>,
    /// Session store port — actively used by the ReAct loop.
    store: Arc<dyn XySessionStore>,
    /// Event sink port — used for compaction / non-queue lifecycle.
    sink: Arc<dyn XyEventSink>,
    /// Steer / follow-up queues + optional active-run EventTx (c525).
    queues: Arc<AsyncQueueRuntime>,
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
        max_iterations: u32,
        compaction_threshold: f64,
        cwd: String,
        compaction_settings: Option<CompactionSettings>,
        model_builder: crate::runtime_protocol::XyModelBuilder,
        permission: Arc<dyn XyPermission>,
        bash_executor: Option<Arc<dyn XyBashExecutor>>,
        export_io: Option<Arc<dyn XyExportIo>>,
        steering_mode: QueueMode,
        follow_up_mode: QueueMode,
    ) -> Self {
        let selected_tools: Vec<String> =
            tool_registry.iter().map(|t| t.name().to_string()).collect();
        let tool_snippets = prompt::collect_tool_snippets(&tool_registry, &selected_tools);

        Self {
            model_manager: ModelManager::new(model_registry, model_builder),
            tools: tool_registry,
            hooks: AgentHooks::empty(),
            tool_mode: XyToolExecutionMode::Sequential,
            system_prompt: system_prompt.clone(),
            session_id: None,
            max_iterations,
            compaction_orchestrator: CompactionOrchestrator::new(
                compaction_threshold,
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
                ..Default::default()
            },
            prompt_templates: Vec::new(),
            extension_commands: Vec::new(),
            bash: crate::agent::session::bash::BashExecHandler::new(bash_executor),
            exporter: crate::agent::session::export::SessionExporter::new(export_io),
            store,
            sink,
            permission,
            queues: Arc::new(AsyncQueueRuntime::new(steering_mode, follow_up_mode)),
        }
    }

    // ── Model management (delegated to ModelManager) ──────────────

    /// Get the current model config.
    pub fn current_model(&self) -> Option<&XyModelMeta> {
        self.model_manager.current_model()
    }

    /// Build the current model instance.
    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, String> {
        self.model_manager.build_current_model()
    }

    /// Get current thinking level (clamped).
    pub fn thinking_level(&self) -> ThinkingLevel {
        self.model_manager.thinking_level()
    }

    /// Set thinking level.
    pub fn set_thinking_level(&mut self, level: ThinkingLevel) {
        self.model_manager.set_thinking_level(level);
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
    }

    /// Select a specific model by ID.
    pub fn select_model(&mut self, model_id: &str) -> Result<(), String> {
        self.model_manager.select_model(model_id)?;
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
        Ok(())
    }

    // ── Prompt templates and commands ───────────────────────────

    /// Register prompt templates discovered by the XyResourceLoader.
    ///
    /// Converts the loader's `PromptTemplate` (content field) into the runtime
    /// `agent::templates::PromptTemplate` (body field) and records the source
    /// path for provenance. Each registered template becomes a `/template:name`
    /// command.
    pub fn register_prompt_commands(
        &mut self,
        templates: &[crate::domain::resource_types::PromptTemplate],
    ) {
        for t in templates {
            self.prompt_templates.push(PromptTemplate {
                name: t.name.clone(),
                description: t.description.clone(),
                source_info: Some(t.source_info.clone()),
            });
        }
    }

    /// Get all available commands (builtin + extension + prompt templates).
    pub(crate) fn get_commands(&self) -> Vec<SlashCommandInfo> {
        let mut all = get_all_commands(&self.extension_commands);
        // Surface registered prompt templates as commands so callers (c110
        // slash dispatch, c115 RPC get_commands) can discover them.
        for t in &self.prompt_templates {
            let mut cmd = SlashCommandInfo::new(
                format!("template:{}", t.name),
                t.description
                    .clone()
                    .unwrap_or_else(|| "prompt template".into()),
                crate::agent::prompt::commands::SlashCommandSource::Prompt,
            );
            if let Some(ref si) = t.source_info {
                cmd.source_info = Some(si.clone());
            }
            all.push(cmd);
        }
        all
    }

    // ── Session management ────────────────────────────────────────

    /// Set the active session ID.
    pub fn set_session(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }

    /// Get the active session ID.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Ensure a session exists (create if needed).
    pub async fn ensure_session(&self, id: &str, parent: Option<&str>) -> Result<(), String> {
        if !self.store.exists(id).await {
            let cwd_clone = self.cwd.clone();
            self.store.create(id, Some(&cwd_clone), parent).await?;
        }
        Ok(())
    }

    /// Load conversation messages from the session store (leaf branch).
    pub(crate) async fn load_conversation_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentMessage>, String> {
        let entries = self.store.load_entries(session_id).await?;
        Ok(entries
            .iter()
            .filter_map(|e| e.as_agent_message())
            .collect())
    }

    /// Shared session store handle (same instance as Driver uses).
    pub fn session_store(&self) -> Arc<dyn XySessionStore> {
        self.store.clone()
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

    pub(crate) fn tool_mode(&self) -> XyToolExecutionMode {
        self.tool_mode
    }

    pub(crate) fn set_tool_mode(&mut self, mode: XyToolExecutionMode) {
        self.tool_mode = mode;
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
        let msg = AgentMessage::user(message);
        self.queues
            .steer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enqueue(msg);
        self.queues.notify_queue_update();
    }

    /// Enqueue a follow-up message (injected when the run would otherwise stop).
    pub fn follow_up(&self, message: impl Into<String>) {
        let msg = AgentMessage::user(message);
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

    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    pub fn model_registry(&self) -> &ModelRegistry {
        self.model_manager.registry()
    }

    /// Current working directory.
    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    // ── Fork ────────────────────────────────────────────────────

    /// Fork the current session at a given entry, creating a child session.
    ///
    /// Returns the child session ID on success. See [`ForkPosition`].
    pub async fn fork_session(
        &self,
        at_entry_id: &str,
        position: crate::domain::session_types::ForkPosition,
    ) -> Result<String, String> {
        let parent_id = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

        let child_id = uuid::Uuid::new_v4().to_string();

        self.store
            .fork(parent_id, &child_id, at_entry_id, position)
            .await
            .map_err(|e| format!("fork failed: {e}"))?;

        Ok(child_id)
    }

    // ── Skill commands ──────────────────────────────────────

    // ── Dynamic system prompt ────────────────────────────────────

    /// Rebuild the system prompt from current options.
    pub fn rebuild_system_prompt(&mut self) {
        self.system_prompt = Some(prompt::build_system_prompt(&self.prompt_opts));
    }

    /// Set the active system prompt text and rebuild.
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.prompt_opts.system_prompt = prompt.clone();
        self.system_prompt = prompt;
        self.rebuild_system_prompt();
    }

    /// Set the active tool set and rebuild the system prompt to reflect it.
    pub fn set_tools(&mut self, tools: ToolSet) {
        self.prompt_opts.selected_tools = tools.iter().map(|t| t.name().to_string()).collect();
        self.prompt_opts.tool_snippets =
            prompt::collect_tool_snippets(&tools, &self.prompt_opts.selected_tools);
        self.tools = tools;
        self.rebuild_system_prompt();
    }

    /// Replace the active hooks.
    pub fn replace_hooks(&mut self, hooks: AgentHooks) {
        self.hooks = hooks;
    }

    /// Set the permission port.
    pub fn set_permission(&mut self, permission: std::sync::Arc<dyn XyPermission>) {
        self.permission = permission;
    }

    // ── Session stats ────────────────────────────────────────────

    /// Get session statistics.
    pub async fn get_session_stats(&self) -> Result<SessionStats, String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        crate::agent::session::stats::compute(self.store.as_ref(), sid).await
    }

    // ── Bash execution (`!cmd` / `!!cmd`) ───────────────────────

    /// Execute a user-initiated bash command and record the result.
    ///
    /// `exclude_from_context=true` (the `!!` prefix) stores the entry on disk
    /// but omits it from LLM context (see `build_session_context`).
    ///
    /// Takes `&self` so an in-flight bash can be cancelled via [`Self::abort_bash`]
    /// / [`crate::agent::AgentRuntime::abort`] without an exclusive borrow.
    pub async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<crate::runtime_protocol::XyBashResult, String> {
        let store: &dyn XySessionStore = self.store.as_ref();
        let sid = self.session_id().map(str::to_string);
        self.bash
            .execute(
                store,
                sid.as_deref(),
                command,
                exclude_from_context,
                chunk_tx,
            )
            .await
    }

    /// Persist a bash result as a `BashExecution` session entry.
    pub async fn record_bash_result(
        &self,
        command: &str,
        result: &crate::runtime_protocol::XyBashResult,
        exclude_from_context: bool,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let sid = match session_id {
            Some(s) => s.to_string(),
            None => self
                .session_id()
                .ok_or_else(|| "no active session".to_string())?
                .to_string(),
        };
        crate::agent::session::bash::record_bash_result(
            self.store.as_ref(),
            command,
            result,
            exclude_from_context,
            &sid,
        )
        .await
    }

    /// Get a reference to the permission engine (injected at construction).
    pub fn get_permission(&self) -> std::sync::Arc<dyn XyPermission> {
        self.permission.clone()
    }

    /// Abort any in-flight bash execution (`&self` so [`crate::agent::AgentRuntime::abort`] can call it).
    pub fn abort_bash(&self) {
        self.bash.abort();
    }

    // ── Lifecycle management ───────────────────────────────────────

    // ── Export / import (delegated to SessionExporter) ─────────

    /// Export the active session's entries to an HTML file. Returns the path.
    pub async fn export_to_html(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let sid = self.session_id().ok_or("no active session")?.to_string();
        self.exporter
            .export_to_html(self.store.as_ref(), &sid, path)
            .await
    }

    /// Export the active session's entries as JSONL. Returns the path.
    pub async fn export_to_jsonl(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let sid = self.session_id().ok_or("no active session")?.to_string();
        self.exporter
            .export_to_jsonl(self.store.as_ref(), &sid, path)
            .await
    }

    /// Import a JSONL file into a brand-new session. Returns the new session id.
    ///
    /// The new session id is derived from the source header (re-used) to keep
    /// identities stable across export/import; the file lands without
    /// overwriting an existing session.
    pub async fn import_from_jsonl(&self, path: &std::path::Path) -> Result<String, String> {
        self.exporter
            .import_from_jsonl(self.store.as_ref(), path)
            .await
    }

    /// Check and perform auto-compaction if the context is full.
    /// Returns true if compaction was performed.
    pub async fn maybe_auto_compact(&self) -> Result<bool, String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

        let model = self
            .build_current_model()
            .map_err(|e| format!("no model: {e}"))?;

        let ctx_window = self
            .current_model()
            .map(|m| m.context_window)
            .unwrap_or(128000);

        self.compaction_orchestrator
            .maybe_auto_compact(
                self.store.as_ref(),
                sid,
                model.as_ref(),
                self.sink.as_ref(),
                ctx_window,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::SessionManager;
    use std::path::PathBuf;

    fn make_session() -> AgentCapabilities {
        let mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: std::sync::Arc<dyn crate::runtime_protocol::XySessionStore> =
            std::sync::Arc::new(mgr);
        let sink: std::sync::Arc<dyn crate::runtime_protocol::XyEventSink> =
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
            50,
            0.8,
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
        )
    }

    fn loader_template(
        name: &str,
        body: &str,
        source: &str,
    ) -> crate::domain::resource_types::PromptTemplate {
        crate::domain::resource_types::PromptTemplate {
            name: name.into(),
            content: body.into(),
            description: None,
            argument_hint: None,
            source_info: SourceInfo {
                path: PathBuf::from(source),
                source: "test".into(),
                scope: SourceScope::Temporary,
                origin: SourceOrigin::TopLevel,
                base_dir: None,
            },
        }
    }

    #[test]
    fn registered_templates_appear_in_commands() {
        let mut session = make_session();
        session.register_prompt_commands(&[
            loader_template("review", "body", "/x/review.md"),
            loader_template("plan", "body2", "/x/plan.md"),
        ]);
        let names: Vec<String> = session.get_commands().into_iter().map(|c| c.name).collect();
        assert!(names.iter().any(|n| n == "template:review"));
        assert!(names.iter().any(|n| n == "template:plan"));
        // All 22 builtin names should also be present.
        assert!(names.iter().any(|n| n == "model"));
        assert!(names.iter().any(|n| n == "export"));
        assert!(names.iter().any(|n| n == "compact"));
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

    #[test]
    fn source_path_preserved_after_registration() {
        let mut session = make_session();
        session.register_prompt_commands(&[loader_template(
            "greet",
            "hi",
            "/home/u/.xylitol/prompts/greet.md",
        )]);
        let t = session
            .prompt_templates
            .iter()
            .find(|t| t.name == "greet")
            .unwrap();
        assert_eq!(
            t.source_info.as_ref().map(|si| si.path.as_path()),
            Some(std::path::Path::new("/home/u/.xylitol/prompts/greet.md"))
        );
    }
}
