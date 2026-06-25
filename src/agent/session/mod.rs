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

mod bash_exec;
mod events;
mod export;
mod io;
mod prompt_result;
mod retry;
mod stats;
mod steering;

pub use self::io::SessionIO;
pub use self::prompt_result::PromptResult;
pub use self::stats::{ContextUsage, SessionStats, estimate_tokens, get_context_usage};

use crate::agent::commands::{SlashCommandInfo, get_all_commands};
use crate::agent::compaction_orchestrator::CompactionOrchestrator;
use crate::agent::model_manager::ModelManager;
use crate::agent::output_guard;
use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::queue::MessageQueue;
use crate::agent::skill_manager::SkillManager;
use crate::agent::templates::{PromptTemplate, is_template_line, parse_template_line};
use crate::agent::tools::ToolRegistry;
use crate::core::traits::XyModel;
use crate::core::types::{ModelMeta, ThinkingLevel};
use crate::infra::event::lifecycle::AgentLifecycleEvent;
use crate::infra::event::{EventBus, UnsubscribeHandle};
use crate::infra::resource::SkillInfo;
use crate::infra::sandbox::{SandboxEngine, SandboxVerdict, noop_engine};
use crate::infra::session::manager::SessionManager;
#[cfg(test)]
use crate::infra::source_info::{SourceInfo, SourceOrigin, SourceScope};

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
    /// Session persistence and navigation.
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
    /// Event bus for lifecycle notifications.
    event_bus: EventBus,
    /// Handle for lifecycle subscription (dropped on unsubscribe/dispose).
    lifecycle_handle: Option<UnsubscribeHandle>,
    /// Auto-retry state machine (None = no retry in progress).
    retry_engine: crate::agent::session::retry::AutoRetryEngine,
    /// Active bash-execution cancellation token (`Some` while a `!`/`!!` runs).
    bash_handler: crate::agent::session::bash_exec::BashExecHandler,

    /// Sandbox engine for tool execution isolation.
    sandbox_engine: Option<std::sync::Arc<dyn SandboxEngine>>,
}

impl AgentSession {
    pub fn new(
        model_registry: ModelRegistry,
        tool_registry: ToolRegistry,
        session_manager: SessionManager,
        system_prompt: Option<String>,
        max_iterations: u32,
        compaction_threshold: f64,
        cwd: String,
    ) -> Self {
        Self {
            model_manager: ModelManager::new(model_registry),
            tool_registry,
            active_tools: Vec::new(),
            session_io: SessionIO::new(session_manager),
            system_prompt,
            session_id: None,
            max_iterations,
            compaction_orchestrator: CompactionOrchestrator::new(compaction_threshold),
            cwd: cwd.clone(),
            prompt_opts: SystemPromptOpts {
                cwd,
                ..Default::default()
            },
            message_queue: MessageQueue::new(),
            prompt_templates: Vec::new(),
            extension_commands: Vec::new(),
            skill_manager: SkillManager::new(),
            event_bus: EventBus::new(),
            lifecycle_handle: None,
            retry_engine: crate::agent::session::retry::AutoRetryEngine::new(),
            bash_handler: crate::agent::session::bash_exec::BashExecHandler::new(),
            sandbox_engine: None,
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
        // Fire-and-forget persistence
        if let Some(ref sid) = self.session_id {
            let io = self.session_io.clone();
            let sid = sid.clone();
            let level_str = level.as_str().to_string();
            tokio::spawn(async move {
                let _ = io.append_thinking_level_change(&sid, &level_str).await;
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
        // Fire-and-forget persistence
        if let Some(ref sid) = self.session_id {
            let io = self.session_io.clone();
            let sid = sid.clone();
            let mid = model_id.to_string();
            tokio::spawn(async move {
                let _ = io.append_model_change(&sid, &mid, &mid).await;
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
        templates: &[crate::infra::resource::PromptTemplate],
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
                crate::agent::commands::SlashCommandSource::Prompt,
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
        if let Some((exclude, command)) = crate::agent::bash_executor::parse_bang_prefix(input)
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
        if let Some(cmd_name) = crate::agent::commands::is_slash_command(input) {
            let all_cmds = self.get_commands();
            if crate::agent::commands::find_command(cmd_name, &all_cmds).is_some() {
                let args = crate::agent::commands::get_command_args(input)
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
        if !self.session_io.manager().exists(id) {
            let cwd_clone = self.cwd.clone();
            self.session_io
                .manager()
                .create(id, Some(&cwd_clone), parent)
                .await?;
        }
        Ok(())
    }

    // ── Accessors ─────────────────────────────────────────────────

    pub(crate) fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    pub fn session_manager(&self) -> &SessionManager {
        self.session_io.manager()
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

    // ── OutputGuard ──────────────────────────────────────────

    /// Enter print mode: take over stdout so agent/tool output is suppressed.
    /// Returns a guard that restores stdout when dropped.
    #[allow(dead_code)]
    pub(crate) fn enter_print_mode(&self) -> output_guard::OutputGuard {
        output_guard::take_over_stdout()
    }

    /// Leave print mode: restore stdout.
    pub fn leave_print_mode(&self) {
        output_guard::restore_stdout();
    }

    /// Check if stdout is currently taken over (print mode active).
    pub fn is_in_print_mode(&self) -> bool {
        output_guard::is_stdout_taken_over()
    }

    // ── Event bus & subscription (methods live in events.rs) ──

    // ── Auto-retry (delegated to AutoRetryEngine) ─────────────

    /// Check whether an assistant message signals a retryable error.
    pub fn _is_retryable_error(msg: &crate::core::message::AgentMessage) -> bool {
        crate::agent::session::retry::AutoRetryEngine::is_retryable_error(msg)
    }

    /// Check whether the session should retry after the agent ends.
    pub fn _will_retry_after_agent_end(&self) -> bool {
        self.retry_engine.will_retry_after_agent_end()
    }

    /// Initialize or reset the retry state for a new agent run.
    pub fn _init_retry_state(&mut self, max_retries: u32, base_delay_ms: u64) {
        self.retry_engine.init_state(max_retries, base_delay_ms);
    }

    /// Prepare and execute a retry attempt.
    pub async fn _prepare_retry(&self) -> bool {
        self.retry_engine.prepare_retry(&self.event_bus).await
    }

    /// Abort any in-progress retry.
    pub fn _abort_retry(&self) {
        self.retry_engine.abort();
    }

    /// Start a new session, creating it in the session manager.
    /// Sets the active session ID and persists model/thinking state.
    pub async fn start_new_session(
        &mut self,
        name: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), String> {
        let id = name
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        self.session_io.create(&id, &self.cwd, parent).await?;
        self.session_id = Some(id.clone());

        // Emit initial model state
        if let Some(model) = self.current_model() {
            let io = self.session_io.clone();
            let sid = id.clone();
            let provider = model.config.provider_name().to_string();
            let model_id = model.config.model.clone();
            tokio::spawn(async move {
                let _ = io.append_model_change(&sid, &provider, &model_id).await;
            });
        }

        Ok(())
    }

    /// Resume an existing session, loading its entries and validating CWD.
    pub async fn resume_session(&mut self, id: &str) -> Result<(), String> {
        // Load + validate CWD
        let entries = self.session_io.load_validated(id, &self.cwd).await?;

        self.session_id = Some(id.to_string());

        // Restore thinking level and model from session entries
        for entry in &entries {
            match entry {
                crate::infra::session::SessionEntry::ThinkingLevelChange(e) => {
                    if let Ok(level) =
                        serde_json::from_value::<ThinkingLevel>(serde_json::json!(e.thinking_level))
                    {
                        self.model_manager.set_thinking_level(level);
                    }
                }
                crate::infra::session::SessionEntry::ModelChange(e) => {
                    // Try to find and select this model
                    let model_id = format!("{}/{}", e.provider, e.model_id);
                    let _ = self.model_manager.select_model(&model_id);
                }
                _ => {}
            }
        }

        Ok(())
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
            .compact(self.session_io.manager(), sid, model, &self.event_bus)
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
        trust_manager: &crate::infra::trust::TrustManager,
        trusted: bool,
    ) -> Result<bool, String> {
        trust_manager.set_trust(&self.cwd, Some(trusted))?;
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

        self.session_io
            .fork(parent_id, &child_id, at_entry_id)
            .await?;

        Ok(child_id)
    }

    /// Navigate the session tree: change the current leaf to a different entry.
    /// After navigation, future appends will create children of the target entry.
    pub fn navigate_tree(&self, target_id: &str) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        self.session_io.navigate(sid, target_id);
        Ok(())
    }

    /// Switch to a different session file.
    pub async fn switch_session(&self, new_path: &str) -> Result<(), String> {
        let new_id = std::path::Path::new(new_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        self.session_io.switch(new_id, new_path).await?;
        self.session_io.manager().set_active_session(new_id);
        Ok(())
    }

    // ── Skill commands ──────────────────────────────────────────

    /// Register skill commands from loaded skills.
    /// When a skill is loaded, `/skill:name` slash command is auto-registered.
    pub fn register_skill_commands(&mut self, _skills: &[crate::infra::resource::SkillInfo]) {
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
        let ctx = self.session_io.manager().build_session_context(sid).await?;

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
    ) -> Result<crate::agent::bash_executor::BashResult, String> {
        let result = self.bash_handler.execute_raw(command).await;

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
        result: &crate::agent::bash_executor::BashResult,
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
            &self.session_io,
            command,
            result,
            exclude_from_context,
            &sid,
        )
        .await
    }

    /// Set the sandbox engine for tool execution isolation.
    pub fn set_sandbox_engine(&mut self, engine: Option<std::sync::Arc<dyn SandboxEngine>>) {
        self.sandbox_engine = engine;
    }

    /// Get a reference to the sandbox engine, or a no-op engine if not set.
    pub fn get_sandbox_engine(&self) -> std::sync::Arc<dyn SandboxEngine> {
        self.sandbox_engine.clone().unwrap_or_else(noop_engine)
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
        self.bash_handler.abort();
    }

    // ── Lifecycle management ───────────────────────────────────────

    /// Dispose of the session, cleaning up all resources.
    ///
    /// Cancels in-flight operations (retry, compaction, bash), unsubscribes
    /// all event listeners, and clears state.
    pub fn dispose(&mut self) {
        // Cancel all in-flight operations
        self._abort_retry();
        self.abort_bash();

        // Unsubscribe lifecycle listeners
        self.lifecycle_handle.take();

        // Clear queues
        self.message_queue.clear();
    }

    /// Abort the current operation and wait for idle.
    ///
    /// Cancels retry backoff, bash execution, and emits an abort event.
    pub fn abort(&mut self) {
        self._abort_retry();
        self.abort_bash();

        // Emit agent_end with aborted reason
        if let Some(ref sid) = self.session_id {
            self.event_bus
                .emit_lifecycle(&AgentLifecycleEvent::AgentEnd {
                    session_id: sid.clone(),
                    reason: "aborted".to_string(),
                });
        }
    }

    /// Send a custom message to the session, with optional turn triggering.
    ///
    /// - `trigger_turn`: if true, immediately trigger a new agent turn
    /// - `deliver_as`: delivery mode ("user" to inject as user message)
    pub async fn send_custom_message(
        &self,
        custom_type: &str,
        _content: serde_json::Value,
        _trigger_turn: bool,
        _deliver_as: Option<&str>,
    ) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        // Forward to session manager for persistence; turn triggering is
        // orchestrated by AgentLoop (c185).
        self.session_io
            .manager()
            .append_custom_message(sid, custom_type, _content, false, None)
            .await
    }

    // ── Export / import (delegated to SessionExporter) ─────────

    /// Export the active session's entries to an HTML file. Returns the path.
    pub async fn export_to_html(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let sid = self.session_id().ok_or("no active session")?.to_string();
        crate::agent::session::export::export_to_html(self.session_io.manager(), &sid, path).await
    }

    /// Export the active session's entries as JSONL. Returns the path.
    pub async fn export_to_jsonl(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let sid = self.session_id().ok_or("no active session")?.to_string();
        crate::agent::session::export::export_to_jsonl(self.session_io.manager(), &sid, path).await
    }

    /// Import a JSONL file into a brand-new session. Returns the new session id.
    pub async fn import_from_jsonl(&self, path: &std::path::Path) -> Result<String, String> {
        crate::agent::session::export::import_from_jsonl(self.session_io.manager(), path).await
    }

    /// Share guidance stub — returns a configuration hint (no network upload).
    pub fn share_as_gist(&self, path: &std::path::Path) -> String {
        crate::agent::session::export::share_as_gist(path)
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
                self.session_io.manager(),
                sid,
                model.as_ref(),
                &self.event_bus,
                ctx_window,
            )
            .await
    }
}

/// Persist a bash result as a `BashExecution` session entry (free helper used
/// by both [`AgentSession::record_bash_result`](super::AgentSession::record_bash_result)
/// and the bash-execution collaborator).
pub(crate) async fn record_bash_result(
    io: &crate::agent::session::io::SessionIO,
    command: &str,
    result: &crate::agent::bash_executor::BashResult,
    exclude_from_context: bool,
    session_id: &str,
) -> Result<(), String> {
    io.manager()
        .append_bash_execution(crate::infra::session::manager::BashExecutionParams {
            session_id,
            command,
            output: &result.output,
            exit_code: result.exit_code,
            cancelled: result.cancelled,
            truncated: result.truncated,
            full_output_path: result.full_output_path.as_deref(),
            exclude_from_context,
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::tools::ToolRegistry;
    use std::path::PathBuf;

    fn make_session() -> AgentSession {
        let mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        AgentSession::new(
            ModelRegistry::new(),
            ToolRegistry::builtins(),
            mgr,
            Some("you are helpful".into()),
            50,
            0.8,
            ".".into(),
        )
    }

    fn loader_template(
        name: &str,
        body: &str,
        source: &str,
    ) -> crate::infra::resource::PromptTemplate {
        crate::infra::resource::PromptTemplate {
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
