//! AgentSession — core agent lifecycle management.
//!
//! Handles:
//! - Model registry and current model tracking
//! - Thinking level toggle (low/medium/high, clamped to model)
//! - Tool registry management
//! - Session persistence integration
//! - Compaction integration
//! - Model switching (cycleForward/cycleBackward/select)
//! - Context token estimation

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

pub(crate) use crate::core::ports::{EventSink, SessionStore};

mod io;
mod prompt_result;
mod stats;
mod steering;

pub use self::io::SessionIO;
pub use self::prompt_result::PromptResult;
pub use self::stats::{ContextUsage, SessionStats, estimate_tokens, get_context_usage};

use crate::agent::compaction::CompactionSettings;
use crate::agent::compaction::orchestrator::CompactionOrchestrator;
use crate::agent::model::manager::ModelManager;
use crate::agent::prompt::commands::{SlashCommandInfo, get_all_commands};
use crate::agent::prompt::skills::SkillManager;
use crate::agent::prompt::templates::{PromptTemplate, is_template_line, parse_template_line};
use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::runtime::MessageQueue;
use crate::agent::tools::ToolRegistry;
use crate::core::bash::parse_bang_prefix;
use crate::core::ports::{BashExecutor, SandboxEngine, SandboxVerdict, TrustStore, XyModel};
use crate::core::resource_types::SkillInfo;
use crate::core::session_types::{
    BashExecutionEntry, EntryBase, ModelChangeEntry, SessionEntry, ThinkingLevelChangeEntry,
};
#[cfg(test)]
use crate::core::source_info::{SourceInfo, SourceOrigin, SourceScope};
use crate::core::types::{ModelMeta, ThinkingLevel};

// ── Model Registry ──────────────────────────────────────────────────

pub use crate::agent::model::registry::ModelRegistry;

// ── AgentSession ────────────────────────────────────────────────────

/// Core agent session — encapsulates model, tools, session persistence, and events.
pub struct AgentSession {
    /// Model management (registry, selection, thinking level).
    model_manager: ModelManager,
    /// Tool registry (all available tools).
    tool_registry: ToolRegistry,
    /// Names of currently active tools (empty = all allowed).
    active_tools: Vec<String>,
    /// Session persistence via the SessionStore port (HC-2). Held for the
    /// ReAct loop to consume load_context/append_entry/exists; the loop
    /// currently builds history inline (c185) and will migrate to this port.
    #[allow(dead_code)]
    session_io: SessionIO,
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
    /// Message queue for steer/followUp.
    #[allow(dead_code)]
    message_queue: MessageQueue,
    /// Registered prompt templates for /template:name expansion.
    prompt_templates: Vec<PromptTemplate>,
    /// Extension-registered slash commands.
    extension_commands: Vec<SlashCommandInfo>,
    /// Skill management (activation, XML expansion).
    skill_manager: SkillManager,
    /// Injected bash executor port (HC-2).
    bash_executor: Arc<dyn BashExecutor>,
    /// Active bash-execution cancellation token (`Some` while a `!`/`!!` runs).
    bash_cancel: Option<CancellationToken>,

    /// Sandbox engine for tool execution isolation (injected at construction).
    sandbox_engine: std::sync::Arc<dyn SandboxEngine>,
    /// Session store port (HC-2) — actively used by the ReAct loop.
    #[allow(dead_code)]
    store: Arc<dyn SessionStore>,
    /// Event sink port (HC-2) — actively used for lifecycle events.
    sink: Arc<dyn EventSink>,
}

impl AgentSession {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model_registry: ModelRegistry,
        tool_registry: ToolRegistry,
        store: Arc<dyn SessionStore>,
        sink: Arc<dyn EventSink>,
        system_prompt: Option<String>,
        max_iterations: u32,
        compaction_threshold: f64,
        cwd: String,
        compaction_settings: Option<CompactionSettings>,
        model_builder: crate::core::ports::ModelBuilder,
        sandbox: Arc<dyn SandboxEngine>,
        bash_executor: Arc<dyn BashExecutor>,
    ) -> Self {
        Self {
            model_manager: ModelManager::new(model_registry, model_builder),
            tool_registry,
            active_tools: Vec::new(),
            session_io: SessionIO::new(store.clone()),
            system_prompt,
            session_id: None,
            max_iterations,
            compaction_orchestrator: CompactionOrchestrator::new(
                compaction_threshold,
                compaction_settings.unwrap_or_default(),
            ),
            cwd: cwd.clone(),
            prompt_opts: SystemPromptOpts {
                cwd,
                ..Default::default()
            },
            message_queue: MessageQueue::new(),
            prompt_templates: Vec::new(),
            extension_commands: Vec::new(),
            skill_manager: SkillManager::new(),
            bash_executor,
            bash_cancel: None,
            store,
            sink,
            sandbox_engine: sandbox,
        }
    }

    // ── Model management (delegated to ModelManager) ──────────────

    /// Get the current model config.
    pub fn current_model(&self) -> Option<&ModelMeta> {
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

    /// Cycle to the next model.
    pub fn cycle_forward(&mut self) -> Option<&ModelMeta> {
        self.model_manager.cycle_forward()
    }

    /// Cycle to the previous model.
    #[allow(dead_code)]
    pub(crate) fn cycle_backward(&mut self) -> Option<&ModelMeta> {
        let len = self.model_manager.registry().len();
        if len == 0 {
            return None;
        }
        let current = self.model_manager.current_index();
        let prev = if current == 0 { len - 1 } else { current - 1 };
        // Cycle forward to wrap around, since ModelManager only has cycle_forward
        for _ in 0..len {
            if self.model_manager.current_index() == prev {
                break;
            }
            self.model_manager.cycle_forward();
        }
        self.model_manager.current_model()
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

    /// Register a prompt template.
    #[allow(dead_code)]
    pub(crate) fn register_template(&mut self, template: PromptTemplate) {
        self.prompt_templates.push(template);
    }

    /// Register prompt templates.
    #[allow(dead_code)]
    pub(crate) fn register_templates(&mut self, templates: Vec<PromptTemplate>) {
        self.prompt_templates.extend(templates);
    }

    /// Register prompt templates discovered by the ResourceLoader.
    ///
    /// Converts the loader's `PromptTemplate` (content field) into the runtime
    /// `agent::templates::PromptTemplate` (body field) and records the source
    /// path for provenance. Each registered template becomes a `/template:name`
    /// command.
    pub fn register_prompt_commands(
        &mut self,
        templates: &[crate::core::resource_types::PromptTemplate],
    ) {
        for t in templates {
            self.prompt_templates.push(PromptTemplate {
                name: t.name.clone(),
                body: t.content.clone(),
                description: t.description.clone(),
                argument_hint: t.argument_hint.clone(),
                source_info: Some(t.source_info.clone()),
            });
        }
    }

    /// Register an extension slash command.
    #[allow(dead_code)]
    pub(crate) fn register_command(&mut self, cmd: SlashCommandInfo) {
        self.extension_commands.push(cmd);
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

    /// Process user input: intercept /commands and /template:name.
    ///
    /// Returns `Some(expanded_text)` if the input was intercepted and should
    /// be sent to the LLM as expanded prompt text (template expansion).
    /// Returns `None` if the input was handled entirely (command dispatched)
    /// or should pass through unchanged.
    ///
    /// The caller should check `result.is_handled()` first:
    /// - `PromptResult::Handled` means the command was dispatched, no LLM call needed.
    /// - `PromptResult::Expanded(text)` means the template was expanded, send `text` to LLM.
    /// - `PromptResult::PassThrough(text)` means normal input, send `text` to LLM.
    pub fn process_prompt(&self, input: &str) -> PromptResult {
        let input = input.trim();

        // Check for `!cmd` / `!!cmd` first (before slash/template handling).
        if let Some((exclude, command)) = parse_bang_prefix(input)
            && !command.is_empty()
        {
            return PromptResult::Bash {
                exclude_from_context: exclude,
                command: command.to_string(),
            };
        }

        // Check for /template:name
        if is_template_line(input) {
            if let Some((name, args)) = parse_template_line(input)
                && let Some(tmpl) = self.prompt_templates.iter().find(|t| t.name == name)
            {
                let expanded = tmpl.expand(&args);
                return PromptResult::Expanded(expanded);
            }
            return PromptResult::PassThrough(input.to_string());
        }

        // Check for /skill:name args — expand into XML block
        if let Some(rest) = input.strip_prefix("/skill:") {
            let (name, args) = if let Some(pos) = rest.find(char::is_whitespace) {
                let (n, a) = rest.split_at(pos);
                (n.trim().to_string(), a.trim().to_string())
            } else {
                (rest.trim().to_string(), String::new())
            };

            if let Some(xml) = self.expand_skill_command(&name, &args) {
                return PromptResult::Expanded(xml);
            }
            // Skill not found: pass through as-is
            return PromptResult::PassThrough(input.to_string());
        }

        // Check for /command
        if let Some(cmd_name) = crate::agent::prompt::commands::is_slash_command(input) {
            let all_cmds = self.get_commands();
            if crate::agent::prompt::commands::find_command(cmd_name, &all_cmds).is_some() {
                let args = crate::agent::prompt::commands::get_command_args(input)
                    .unwrap_or("")
                    .to_string();
                return PromptResult::Handled {
                    command: cmd_name.to_string(),
                    args,
                };
            }
        }

        // Normal pass-through
        PromptResult::PassThrough(input.to_string())
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

    // ── Accessors ─────────────────────────────────────────────────

    pub(crate) fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    pub fn compaction_threshold(&self) -> f64 {
        self.compaction_orchestrator.threshold()
    }

    /// Compaction tuning in use (reserve / keep-recent tokens, master toggle).
    pub fn compaction_settings(&self) -> &CompactionSettings {
        self.compaction_orchestrator.settings()
    }

    pub fn model_registry(&self) -> &ModelRegistry {
        self.model_manager.registry()
    }

    /// Clone the model registry (needed by RPC mode).
    pub fn registry_clone(&self) -> ModelRegistry {
        self.model_manager.registry_clone()
    }

    /// Current working directory.
    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    // ── Compaction ───────────────────────────────────────────────

    /// Compact the current session, summarizing old entries via LLM.
    ///
    /// Requires an active session and a configured model. Writes a
    /// Compact the current session, emitting lifecycle events.
    pub async fn compact_current_session(&self, model: &dyn XyModel) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        self.compaction_orchestrator
            .compact(self.store.as_ref(), sid, model, self.sink.as_ref())
            .await
    }

    // ── Skills (delegated to SkillManager) ─────────

    pub fn set_skills(&mut self, skills: Vec<SkillInfo>) {
        self.skill_manager.set_skills(skills);
    }

    pub fn expand_skill_command(&self, skill_name: &str, args: &str) -> Option<String> {
        self.skill_manager.expand_command(skill_name, args)
    }

    // ── Project trust (spec c255 / t6) ─────────────────────────

    /// Persist a project trust decision for the current CWD via the trust
    /// store (single source of truth). Used by the `/trust` and `/no-trust`
    /// commands; the decision takes effect on the next resolution / restart.
    /// Returns the persisted decision.
    pub fn save_trust_decision(
        &self,
        trust_store: &dyn TrustStore,
        trusted: bool,
    ) -> Result<bool, String> {
        trust_store.set_trust(&self.cwd, Some(trusted))?;
        Ok(trusted)
    }

    // ── Fork ────────────────────────────────────────────────────

    /// Fork the current session at a given entry, creating a child session.
    ///
    /// Returns the child session ID on success.
    pub async fn fork_session(&self, at_entry_id: &str) -> Result<String, String> {
        let parent_id = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

        let child_id = uuid::Uuid::new_v4().to_string();

        self.store
            .fork(parent_id, &child_id, at_entry_id)
            .await
            .map_err(|e| format!("fork failed: {e}"))?;

        Ok(child_id)
    }

    // ── Skill commands ──────────────────────────────────────

    /// Register skill commands from loaded skills.
    /// When a skill is loaded, `/skill:name` slash command is auto-registered.
    pub fn register_skill_commands(&mut self, _skills: &[crate::core::resource_types::SkillInfo]) {
        let cmds = self.skill_manager.register_commands();
        self.extension_commands.extend(cmds);
    }

    // ── Steering / Follow-up queue ─────────────────────────────
    // (methods live in steering.rs)

    // ── Dynamic system prompt ────────────────────────────────────

    /// Set active tools by name and rebuild the system prompt.
    pub fn set_active_tools(&mut self, tool_names: &[String]) {
        self.active_tools = tool_names.to_vec();
        self.prompt_opts.selected_tools = tool_names.to_vec();
        self.prompt_opts.tool_snippets =
            prompt::collect_tool_snippets(&self.tool_registry, tool_names);
        self.rebuild_system_prompt();
    }

    /// Rebuild the system prompt from current options.
    pub fn rebuild_system_prompt(&mut self) {
        self.system_prompt = Some(prompt::build_system_prompt(&self.prompt_opts));
    }

    /// Set append system prompt text.
    pub fn set_append_prompt(&mut self, text: Option<String>) {
        self.prompt_opts.append_prompt = text;
        self.rebuild_system_prompt();
    }

    // ── Message queue accessors ──────────────────────────────────

    #[allow(dead_code)]
    pub(crate) fn message_queue(&self) -> &MessageQueue {
        &self.message_queue
    }

    #[allow(dead_code)]
    pub(crate) fn message_queue_mut(&mut self) -> &mut MessageQueue {
        &mut self.message_queue
    }

    // ── Session stats ────────────────────────────────────────────

    /// Get session statistics.
    pub async fn get_session_stats(&self) -> Result<SessionStats, String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        let ctx = self.store.build_session_context(sid).await?;

        let user_messages = ctx
            .messages
            .iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
            .count();
        let assistant_messages = ctx
            .messages
            .iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"))
            .count();
        let total_messages = ctx.messages.len();

        Ok(SessionStats {
            session_id: sid.to_string(),
            user_messages,
            assistant_messages,
            total_messages,
            thinking_level: ctx.thinking_level,
            model: ctx.model,
        })
    }

    // ── Bash execution (`!cmd` / `!!cmd`) ───────────────────────

    /// Execute a user-initiated bash command and record the result.
    ///
    /// `exclude_from_context=true` (the `!!` prefix) stores the entry on disk
    /// but omits it from LLM context (see `build_session_context`).
    pub async fn execute_bash(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<crate::core::ports::BashResult, String> {
        let cancel = CancellationToken::new();
        self.bash_cancel = Some(cancel.clone());

        let result = self.bash_executor.execute(command, Some(cancel)).await;

        self.bash_cancel = None;

        // Record on disk.
        if let Some(sid) = self.session_id() {
            let sid = sid.to_string();
            self.record_bash_result(command, &result, exclude_from_context, Some(&sid))
                .await?;
        }

        Ok(result)
    }

    /// Persist a bash result as a `BashExecution` session entry.
    pub async fn record_bash_result(
        &self,
        command: &str,
        result: &crate::core::ports::BashResult,
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
        record_bash_result(
            self.store.as_ref(),
            command,
            result,
            exclude_from_context,
            &sid,
        )
        .await
    }

    /// Get a reference to the sandbox engine (injected at construction).
    pub fn get_sandbox_engine(&self) -> std::sync::Arc<dyn SandboxEngine> {
        self.sandbox_engine.clone()
    }

    /// Check whether a file read is allowed by the sandbox.
    pub fn check_sandbox_read(&self, path: &str) -> SandboxVerdict {
        self.get_sandbox_engine().check_read(path)
    }

    /// Check whether a file write is allowed by the sandbox.
    pub fn check_sandbox_write(&self, path: &str) -> SandboxVerdict {
        self.get_sandbox_engine().check_write(path)
    }

    /// Check whether a network request is allowed by the sandbox.
    pub fn check_sandbox_network(&self, domain: &str) -> SandboxVerdict {
        self.get_sandbox_engine().check_network(domain)
    }

    /// Abort any in-flight bash execution.
    pub fn abort_bash(&mut self) {
        if let Some(cancel) = self.bash_cancel.take() {
            cancel.cancel();
        }
    }

    // ── Lifecycle management ───────────────────────────────────────

    /// Dispose of the session, cleaning up all resources.
    ///
    /// Cancels in-flight bash execution, unsubscribes all event listeners,
    /// and clears state.
    pub fn dispose(&mut self) {
        // Cancel all in-flight operations
        self.abort_bash();

        // Clear queues
        self.message_queue.drain();
    }

    /// Abort the current operation.
    ///
    /// Cancels in-flight bash execution. (Lifecycle emission removed: the
    /// in-process EventBus had zero subscribers — `subscribe` was dead API.
    /// If abort notifications are needed later, extend
    /// `core::ports::LifecycleEvent` and emit via the `EventSink` port.)
    pub fn abort(&mut self) {
        self.abort_bash();
    }

    // ── Export / import (delegated to SessionExporter) ─────────

    /// Export the active session's entries to an HTML file. Returns the path.
    pub async fn export_to_html(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let sid = self.session_id().ok_or("no active session")?.to_string();
        let entries = self.store.load_entries(&sid).await?;
        let html = crate::core::session_export::render_html(&sid, &entries);
        crate::core::session_export::write_to(path, &html)?;
        Ok(path.to_path_buf())
    }

    /// Export the active session's entries as JSONL. Returns the path.
    pub async fn export_to_jsonl(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let sid = self.session_id().ok_or("no active session")?.to_string();
        let entries = self.store.load_entries(&sid).await?;
        let jsonl = crate::core::session_export::render_jsonl(&entries)?;
        crate::core::session_export::write_to(path, &jsonl)?;
        Ok(path.to_path_buf())
    }

    /// Import a JSONL file into a brand-new session. Returns the new session id.
    ///
    /// The new session id is derived from the source header (re-used) to keep
    /// identities stable across export/import; the file lands without
    /// overwriting an existing session.
    pub async fn import_from_jsonl(&self, path: &std::path::Path) -> Result<String, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let entries = crate::core::session_export::parse_jsonl(&bytes)?;
        let new_id = match entries.first() {
            Some(SessionEntry::Header(h)) => h.id.clone(),
            _ => return Err("import: missing header".into()),
        };
        if self.store.exists(&new_id).await {
            return Err(format!("session already exists: {new_id}"));
        }
        for entry in &entries {
            self.store.append_session_entry(&new_id, entry).await?;
        }
        Ok(new_id)
    }

    /// Share guidance stub — returns a configuration hint (no network upload).
    pub fn share_as_gist(&self, path: &std::path::Path) -> String {
        crate::core::session_export::share_guidance_message(path)
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

/// Persist a bash result as a `BashExecution` session entry (free helper used
/// by both [`AgentSession::record_bash_result`](super::AgentSession::record_bash_result)
/// and the bash-execution collaborator).
pub(crate) async fn record_bash_result(
    store: &dyn SessionStore,
    command: &str,
    result: &crate::core::ports::BashResult,
    exclude_from_context: bool,
    session_id: &str,
) -> Result<(), String> {
    let entry = SessionEntry::BashExecution(BashExecutionEntry {
        base: EntryBase {
            entry_type: "bash".into(),
            id: String::new(),
            parent_id: None,
            timestamp: String::new(),
        },
        command: command.to_string(),
        output: result.output.clone(),
        exit_code: result.exit_code,
        cancelled: result.cancelled,
        truncated: result.truncated,
        full_output_path: result.full_output_path.clone(),
        exclude_from_context,
    });
    store.append_session_entry(session_id, &entry).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::SessionManager;
    use std::path::PathBuf;

    fn make_session() -> AgentSession {
        let mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: std::sync::Arc<dyn crate::core::ports::SessionStore> = std::sync::Arc::new(mgr);
        let sink: std::sync::Arc<dyn crate::core::ports::EventSink> =
            std::sync::Arc::new(crate::infra::event::EventBus::new());
        AgentSession::new(
            ModelRegistry::new(std::sync::Arc::new(
                crate::infra::config::value::InfraSecretResolver::new(),
            )),
            ToolRegistry::from_tools(crate::infra::tools::default_tools()),
            store,
            sink,
            Some("you are helpful".into()),
            50,
            0.8,
            ".".into(),
            None,
            std::sync::Arc::new(crate::infra::provider::factory::build_provider),
            crate::infra::sandbox::noop_engine(),
            std::sync::Arc::new(crate::infra::bash_exec::InfraBashExecutor::new()),
        )
    }

    fn loader_template(
        name: &str,
        body: &str,
        source: &str,
    ) -> crate::core::resource_types::PromptTemplate {
        crate::core::resource_types::PromptTemplate {
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
    fn register_prompt_commands_injects_templates() {
        let mut session = make_session();
        session.register_prompt_commands(&[loader_template(
            "review",
            "Review: $1",
            "/home/u/.xylitol/prompts/review.md",
        )]);
        // Wired: process_prompt expands /template:review.
        let result = session.process_prompt("/template:review main.rs");
        match result {
            PromptResult::Expanded(text) => assert!(text.contains("Review: main.rs")),
            other => panic!("expected Expanded, got {other:?}"),
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

    #[test]
    fn positional_args_still_substituted() {
        let mut session = make_session();
        session.register_prompt_commands(&[loader_template(
            "multi",
            "a=$1 b=$@ d=${2:-x}",
            "/x/multi.md",
        )]);
        let result = session.process_prompt("/template:multi foo bar");
        match result {
            PromptResult::Expanded(text) => {
                assert!(text.contains("a=foo"));
                assert!(text.contains("b=foo bar"));
                assert!(text.contains("d=bar"));
            }
            other => panic!("expected Expanded, got {other:?}"),
        }
    }
}
