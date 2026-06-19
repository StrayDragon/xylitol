//! AgentSession — core agent lifecycle management.
//!
//! Aligns with pi's AgentSession class. Handles:
//! - Model registry and current model tracking
//! - Thinking level toggle (low/medium/high, clamped to model)
//! - Tool registry management
//! - Session persistence integration
//! - Compaction integration
//! - Model switching (cycleForward/cycleBackward/select)
//! - Context token estimation

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::agent::commands::{DispatchResult, SlashCommandInfo, get_all_commands};
use crate::agent::model::ModelConfig;
use crate::agent::output_guard;
use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::queue::MessageQueue;
use crate::agent::templates::{PromptTemplate, is_template_line, parse_template_line};
use crate::agent::tools::ToolRegistry;
use crate::agent::traits::XyModel;
use crate::agent::types::{XyContent, XyPart};
use crate::infra::session::compaction::{CompactionSettings, compact_session};
use crate::infra::session::manager::SessionManager;

// ── Thinking Level ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
}

impl ThinkingLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThinkingLevel::Off => "off",
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
        }
    }

    /// Clamp to what the model supports.
    pub fn clamp(self, model_supports_thinking: bool) -> Self {
        if !model_supports_thinking {
            return ThinkingLevel::Off;
        }
        self
    }
}

// ── Model Registry ──────────────────────────────────────────────────

pub use crate::agent::registry::ModelRegistry;

#[derive(Debug, Clone)]
pub struct ModelMeta {
    pub id: String,
    pub config: ModelConfig,
    pub display_name: String,
    pub thinking: bool,
    pub context_window: u64,
}

// ── AgentSession ────────────────────────────────────────────────────

/// Core agent session — encapsulates model, tools, session persistence, and events.
pub struct AgentSession {
    /// Model registry for model switching.
    model_registry: ModelRegistry,
    /// Current model index in the registry.
    current_model_index: usize,
    /// Tool registry with builtin tools.
    tool_registry: ToolRegistry,
    /// Session persistence manager.
    session_manager: SessionManager,
    /// Current thinking level.
    thinking_level: ThinkingLevel,
    /// System prompt to prepend to every turn.
    system_prompt: Option<String>,
    /// Current session ID.
    session_id: Option<String>,
    /// Max ReAct loop iterations per turn.
    max_iterations: u32,
    /// Context window threshold for compaction (0.0-1.0).
    compaction_threshold: f64,
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
    /// Event bus for turn lifecycle notifications (lazy init).
    event_bus: Option<crate::agent::event::AgentEventBus>,
    /// Active bash-execution cancellation token (`Some` while a `!`/`!!` runs).
    bash_cancel: Option<tokio_util::sync::CancellationToken>,
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
            model_registry,
            current_model_index: 0,
            tool_registry,
            session_manager,
            thinking_level: ThinkingLevel::default(),
            system_prompt,
            session_id: None,
            max_iterations,
            compaction_threshold,
            cwd: cwd.clone(),
            prompt_opts: SystemPromptOpts {
                cwd,
                ..Default::default()
            },
            message_queue: MessageQueue::new(),
            prompt_templates: Vec::new(),
            extension_commands: Vec::new(),
            event_bus: None,
            bash_cancel: None,
        }
    }

    // ── Model management ──────────────────────────────────────────

    /// Get the current model config.
    pub fn current_model(&self) -> Option<&ModelMeta> {
        self.model_registry.get_at(self.current_model_index)
    }

    /// Build the current model instance.
    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, String> {
        let meta = self
            .current_model()
            .ok_or_else(|| "no model configured".to_string())?;
        meta.config.build()
    }

    /// Get current thinking level (clamped).
    pub fn thinking_level(&self) -> ThinkingLevel {
        let supports = self.current_model().map(|m| m.thinking).unwrap_or(false);
        self.thinking_level.clamp(supports)
    }

    /// Set thinking level.
    pub fn set_thinking_level(&mut self, level: ThinkingLevel) {
        self.thinking_level = level;
        // Fire-and-forget persistence (async call from sync context OK in tokio tests)
        if let Some(ref sid) = self.session_id {
            let mgr = self.session_manager.clone();
            let sid = sid.clone();
            let level_str = level.as_str().to_string();
            tokio::spawn(async move {
                let _ = mgr.append_thinking_level_change(&sid, &level_str).await;
            });
        }
    }

    /// Cycle to the next model.
    pub fn cycle_forward(&mut self) -> Option<&ModelMeta> {
        if self.model_registry.is_empty() {
            return None;
        }
        self.current_model_index = (self.current_model_index + 1) % self.model_registry.len();
        self.current_model()
    }

    /// Cycle to the previous model.
    #[allow(dead_code)]
    pub(crate) fn cycle_backward(&mut self) -> Option<&ModelMeta> {
        if self.model_registry.is_empty() {
            return None;
        }
        self.current_model_index = if self.current_model_index == 0 {
            self.model_registry.len() - 1
        } else {
            self.current_model_index - 1
        };
        self.current_model()
    }

    /// Select a specific model by ID.
    pub fn select_model(&mut self, model_id: &str) -> Result<(), String> {
        let idx = self
            .model_registry
            .index_of(model_id)
            .ok_or_else(|| format!("model not found: {model_id}"))?;
        self.current_model_index = idx;
        // Fire-and-forget persistence
        if let Some(ref sid) = self.session_id {
            let mgr = self.session_manager.clone();
            let sid = sid.clone();
            let mid = model_id.to_string();
            tokio::spawn(async move {
                let _ = mgr.append_model_change(&sid, &mid, &mid).await;
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
                source_path: Some(t.source_path.clone()),
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
            if let Some(ref path) = t.source_path {
                cmd.source_path = Some(path.clone());
            }
            all.push(cmd);
        }
        all
    }

    /// Routes the behaviour of a recognised slash command into the concrete
    /// handler on AgentSession. The caller (AgentLoop or RPC mode) must hold
    /// a mutably borrowed session to call async handlers.
    ///
    /// Returns [`DispatchResult`] synchronously so the caller can decide
    /// whether to break a loop, fall through to LLM, or show a guidance message.
    pub(crate) fn dispatch_slash_command(&mut self, name: &str, _args: &str) -> DispatchResult {
        if crate::agent::commands::is_tui_command(name) {
            return DispatchResult::NotAvailable {
                reason: format!("`/{name}` requires TUI mode"),
            };
        }

        // Commands whose dispatch is purely synchronous and doesn't need
        // AgentSession state mutations (delegated to the caller loop).
        match name {
            "quit" => {
                // The caller (AgentLoop / RPC) will break the loop.
                DispatchResult::Handled
            }
            _ if crate::agent::commands::has_handler(name) => DispatchResult::Handled,
            _ => DispatchResult::NotFound,
        }
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

        // Check for /command
        if let Some(cmd_name) = crate::agent::commands::is_slash_command(input) {
            let all_cmds = self.get_commands();
            if crate::agent::commands::find_command(cmd_name, &all_cmds).is_some() {
                let args = crate::agent::commands::get_command_args(input)
                    .unwrap_or("")
                    .to_string();
                // Check for TUI-only commands before returning Handled.
                if crate::agent::commands::is_tui_command(cmd_name) {
                    return PromptResult::Expanded(format!(
                        "`/{cmd_name}` requires TUI mode. Run the agent interactively."
                    ));
                }
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
        if !self.session_manager.exists(id) {
            let cwd_clone = self.cwd.clone();
            self.session_manager
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
        &self.session_manager
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    pub fn compaction_threshold(&self) -> f64 {
        self.compaction_threshold
    }

    pub fn model_registry(&self) -> &ModelRegistry {
        &self.model_registry
    }

    /// Clone the model registry (needed by RPC mode).
    pub fn registry_clone(&self) -> ModelRegistry {
        self.model_registry.clone()
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

    // ── Session lifecycle ────────────────────────────────────

    /// Ensure the event bus exists (lazy init).
    #[allow(dead_code)]
    pub(crate) fn ensure_event_bus(&mut self) -> &mut crate::agent::event::AgentEventBus {
        self.event_bus
            .get_or_insert_with(|| crate::agent::event::AgentEventBus::new(64))
    }

    /// Subscribe to agent events.
    #[allow(dead_code)]
    pub(crate) fn subscribe_events(&self) -> Option<crate::agent::event::UnsubscribeHandle> {
        self.event_bus.as_ref().map(|bus| bus.subscribe())
    }

    /// Emit a turn_start event.
    pub fn begin_turn(&self, turn_index: u32) {
        if let Some(ref bus) = self.event_bus {
            bus.emit(crate::agent::r#loop::AgentEvent::TurnStart { turn_index });
        }
    }

    /// Emit a turn_end event.
    pub fn end_turn(&self, turn_index: u32) {
        if let Some(ref bus) = self.event_bus {
            bus.emit(crate::agent::r#loop::AgentEvent::TurnEnd { turn_index });
        }
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

        self.session_manager
            .create(&id, Some(&self.cwd), parent)
            .await?;
        self.session_id = Some(id.clone());

        // Emit initial model state
        if let Some(model) = self.current_model() {
            let mgr = self.session_manager.clone();
            let sid = id.clone();
            let provider = model.config.provider_name().to_string();
            let model_id = model.config.model.clone();
            tokio::spawn(async move {
                let _ = mgr.append_model_change(&sid, &provider, &model_id).await;
            });
        }

        Ok(())
    }

    /// Resume an existing session, loading its entries and validating CWD.
    pub async fn resume_session(&mut self, id: &str) -> Result<(), String> {
        // Load + validate CWD
        let entries = self.session_manager.load_validated(id, &self.cwd).await?;

        self.session_id = Some(id.to_string());

        // Restore thinking level and model from session entries
        for entry in &entries {
            match entry {
                crate::infra::session::SessionEntry::ThinkingLevelChange(e) => {
                    if let Ok(level) =
                        serde_json::from_value::<ThinkingLevel>(serde_json::json!(e.thinking_level))
                    {
                        self.thinking_level = level;
                    }
                }
                crate::infra::session::SessionEntry::ModelChange(e) => {
                    // Try to find and select this model
                    let model_id = format!("{}/{}", e.provider, e.model_id);
                    if let Some(i) = self.model_registry.index_of(&model_id) {
                        self.current_model_index = i;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Get a reference to the event bus if initialized.
    #[allow(dead_code)]
    pub(crate) fn event_bus(&self) -> Option<&crate::agent::event::AgentEventBus> {
        self.event_bus.as_ref()
    }

    // ── Compaction ───────────────────────────────────────────────

    /// Compact the current session, summarizing old entries via LLM.
    ///
    /// Requires an active session and a configured model. Writes a
    /// CompactionEntry to the session file.
    pub async fn compact_current_session(&self, model: &dyn XyModel) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };

        compact_session(&self.session_manager, sid, model, &settings)
            .await
            .map_err(|e| format!("compaction failed: {e}"))?;

        Ok(())
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

        self.session_manager
            .fork(parent_id, &child_id, at_entry_id)
            .await
            .map_err(|e| format!("fork failed: {e}"))?;

        Ok(child_id)
    }

    /// Navigate the session tree: change the current leaf to a different entry.
    /// After navigation, future appends will create children of the target entry.
    pub fn navigate_tree(&self, target_id: &str) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        self.session_manager.navigate_tree(sid, Some(target_id));
        Ok(())
    }

    /// Switch to a different session file.
    pub async fn switch_session(&self, new_path: &str) -> Result<(), String> {
        let new_id = std::path::Path::new(new_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        self.session_manager
            .switch_session(new_id, new_path)
            .await?;
        self.session_manager.set_active_session(new_id);
        Ok(())
    }

    // ── Skill commands ──────────────────────────────────────────

    /// Register skill commands from loaded skills.
    /// When a skill is loaded, `/skill:name` slash command is auto-registered.
    pub fn register_skill_commands(&mut self, skills: &[crate::infra::resource::SkillInfo]) {
        for skill in skills {
            let name = skill.name.clone();
            let desc = skill.description.clone().unwrap_or_default();
            self.extension_commands.push(SlashCommandInfo::new(
                format!("skill:{name}"),
                format!("Activate skill: {desc}"),
                crate::agent::commands::SlashCommandSource::Skill,
            ));
        }
    }

    // ── Extension tool wrapping ─────────────────────────────────

    /// Wrap built-in tools with extension hooks (before_tool_call / after_tool_call).
    ///
    /// Each built-in tool is wrapped so that before execution, extensions can block it,
    /// and after execution, extensions can modify the result.
    #[allow(dead_code)]
    pub(crate) fn wrap_registered_tools(
        &mut self,
        _loader: &crate::agent::extensions::ExtensionLoader,
    ) {
        // Tool wrapping happens at the ToolRegistry level in c80.
        // For now, this is a placeholder that will be wired in AgentLoop.
    }

    /// Check if a steering message is pending (for steering_mode).
    pub fn has_pending_steer(&self) -> bool {
        self.message_queue.has_pending()
    }

    /// Enter steering mode: queue messages while agent is running.
    /// Steering messages don't trigger new turns but are injected into context.
    #[allow(dead_code)]
    pub(crate) fn queue_steer_message(&mut self, message: XyContent) {
        self.message_queue.push(message);
    }

    /// Enter follow-up mode: queue messages to be delivered after current turn.
    #[allow(dead_code)]
    pub(crate) fn queue_follow_up(&mut self, message: XyContent) {
        self.message_queue.push(message);
    }

    /// Drain queued messages for the next turn.
    pub fn drain_queued_messages(&mut self) -> Vec<XyContent> {
        self.message_queue.drain()
    }

    // ── Dynamic system prompt ────────────────────────────────────

    /// Set active tools by name and rebuild the system prompt.
    pub fn set_active_tools(&mut self, tool_names: &[String]) {
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

    // ── Message queue ────────────────────────────────────────────

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
        let ctx = self.session_manager.build_session_context(sid).await?;

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

    /// Send a custom message to the session.
    pub async fn send_custom_message(
        &self,
        custom_type: &str,
        content: serde_json::Value,
        display: bool,
    ) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;
        self.session_manager
            .append_custom_message(sid, custom_type, content, display, None)
            .await
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
        let cancel = tokio_util::sync::CancellationToken::new();
        self.bash_cancel = Some(cancel.clone());

        let result = crate::agent::bash_executor::execute(
            command,
            crate::agent::bash_executor::BashExecutorOptions {
                cancel: Some(cancel),
                ..Default::default()
            },
        )
        .await;

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
        self.session_manager
            .append_bash_execution(
                &sid,
                command,
                &result.output,
                result.exit_code,
                result.cancelled,
                result.truncated,
                result.full_output_path.as_deref(),
                exclude_from_context,
            )
            .await
    }

    /// Abort any in-flight bash execution.
    pub fn abort_bash(&mut self) {
        if let Some(cancel) = self.bash_cancel.take() {
            cancel.cancel();
        }
    }

    // ── Export / import ─────────────────────────────────────────

    /// Export the active session's entries to an HTML file. Returns the path.
    pub async fn export_to_html(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let sid = self.session_id().ok_or("no active session")?.to_string();
        let entries = self.session_manager.load(&sid).await?;
        let html = crate::infra::session::export::render_html(&sid, &entries);
        crate::infra::session::export::write_to(path, &html)?;
        Ok(path.to_path_buf())
    }

    /// Export the active session's entries as JSONL. Returns the path.
    pub async fn export_to_jsonl(
        &self,
        path: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let sid = self.session_id().ok_or("no active session")?.to_string();
        let entries = self.session_manager.load(&sid).await?;
        let jsonl = crate::infra::session::export::render_jsonl(&entries)?;
        crate::infra::session::export::write_to(path, &jsonl)?;
        Ok(path.to_path_buf())
    }

    /// Import a JSONL file into a brand-new session. Returns the new session id.
    ///
    /// The new session id is derived from the source header (re-used) to keep
    /// identities stable across export/import, but the file lands in this
    /// manager's sessions dir without overwriting an existing session.
    pub async fn import_from_jsonl(&self, path: &std::path::Path) -> Result<String, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let entries = crate::infra::session::export::parse_jsonl(&bytes)?;
        let new_id = match entries.first() {
            Some(crate::infra::session::SessionEntry::Header(h)) => h.id.clone(),
            _ => return Err("import: missing header".into()),
        };
        if self.session_manager.exists(&new_id) {
            return Err(format!("session already exists: {new_id}"));
        }
        // Append all entries into a fresh session file.
        for entry in &entries {
            self.session_manager.append(&new_id, entry).await?;
        }
        Ok(new_id)
    }

    /// Share guidance stub — returns a configuration hint (no network upload).
    pub fn share_as_gist(&self, path: &std::path::Path) -> String {
        crate::infra::session::export::share_guidance_message(path)
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

        let session_ctx = self.session_manager.build_session_context(sid).await?;
        let token_estimate: u64 = session_ctx
            .messages
            .iter()
            .map(|m| (m.to_string().len() as u64).div_ceil(4))
            .sum();

        if !should_compact(token_estimate, ctx_window, self.compaction_threshold) {
            return Ok(false);
        }

        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };

        compact_session(&self.session_manager, sid, model.as_ref(), &settings)
            .await
            .map_err(|e| format!("auto-compaction: {e}"))?;

        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub total_messages: usize,
    pub thinking_level: String,
    pub model: Option<(String, String)>,
}

// ── Context estimation ──────────────────────────────────────────────

/// Estimate token count from messages using simple heuristic (1 token ≈ 4 chars).
pub fn estimate_tokens(messages: &[XyContent]) -> u64 {
    let mut total = 0u64;
    for msg in messages {
        for part in &msg.parts {
            match part {
                XyPart::Text(s) | XyPart::Thinking(s) => {
                    total += (s.len() as u64).div_ceil(4);
                }
                XyPart::FunctionCall { name, args, id: _ } => {
                    total += (name.len() as u64).div_ceil(4);
                    total += (args.to_string().len() as u64).div_ceil(4);
                }
                XyPart::FunctionResponse {
                    name: _,
                    result,
                    id: _,
                } => {
                    total += (result.len() as u64).div_ceil(4);
                }
            }
        }
    }
    total
}

/// Check if compaction should be triggered.
pub fn should_compact(token_estimate: u64, context_window: u64, threshold: f64) -> bool {
    if context_window == 0 {
        return false;
    }
    let usage_ratio = token_estimate as f64 / context_window as f64;
    usage_ratio >= threshold
}

#[derive(Debug, Clone)]
pub struct ContextUsage {
    pub tokens: u64,
    pub context_window: u64,
    pub percent: u64,
    pub should_compact: bool,
}

/// Get context usage info for the current session state.
pub fn get_context_usage(token_estimate: u64, context_window: u64, threshold: f64) -> ContextUsage {
    let percent = if context_window > 0 {
        ((token_estimate as f64 / context_window as f64) * 100.0) as u64
    } else {
        0
    };
    ContextUsage {
        tokens: token_estimate,
        context_window,
        percent,
        should_compact: should_compact(token_estimate, context_window, threshold),
    }
}

// ── Prompt Result ───────────────────────────────────────────────────

/// Result of processing user input through the prompt interceptor.
#[derive(Debug, Clone)]
pub enum PromptResult {
    /// A slash command was matched and handled. No LLM call needed.
    Handled { command: String, args: String },
    /// A /template:name was expanded. The caller should send the content to the LLM.
    Expanded(String),
    /// A `!cmd` / `!!cmd` bash execution request. The caller should invoke
    /// `execute_bash` with the parsed command; `exclude_from_context` reflects
    /// the bang prefix.
    Bash {
        exclude_from_context: bool,
        command: String,
    },
    /// Normal input — pass through to LLM unchanged.
    PassThrough(String),
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
            source_path: PathBuf::from(source),
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
    fn tui_commands_return_guidance_in_non_tui_mode() {
        let session = make_session();
        let result = session.process_prompt("/settings");
        match result {
            PromptResult::Expanded(text) => assert!(text.contains("TUI")),
            other => panic!("expected Expanded for TUI command, got {other:?}"),
        }
        let result = session.process_prompt("/hotkeys");
        match result {
            PromptResult::Expanded(text) => assert!(text.contains("TUI")),
            other => panic!("expected Expanded for TUI command, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_slash_command_routes_correctly() {
        let mut session = make_session();
        // Known handler
        match session.dispatch_slash_command("compact", "") {
            DispatchResult::Handled => {}
            other => panic!("expected Handled, got {other:?}"),
        }
        // TUI command
        match session.dispatch_slash_command("settings", "") {
            DispatchResult::NotAvailable { ref reason } => assert!(reason.contains("TUI")),
            other => panic!("expected NotAvailable, got {other:?}"),
        }
        // Unknown command
        match session.dispatch_slash_command("nonexistent", "") {
            DispatchResult::NotFound => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
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
            t.source_path.as_deref(),
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
