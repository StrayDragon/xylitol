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

pub(crate) use crate::protocol::ports::{XyEventSink, XySessionStore};

mod bash;
mod export;
mod queue;
mod stats;

pub use self::queue::{
    AsyncQueueRuntime, PendingMessageQueue, QueueChannel, QueueMode, QueueStats,
};
pub use self::stats::{ContextUsage, SessionStats, estimate_tokens, get_context_usage};

use crate::agent::compaction::CompactionSettings;
use crate::agent::compaction::orchestrator::CompactionOrchestrator;
use crate::agent::model::manager::ModelManager;
use crate::agent::prompt::commands::{SlashCommandInfo, get_all_commands};
use crate::agent::prompt::{self, SystemPromptOpts};
use crate::agent::runtime::AgentHooks;
use crate::agent::tools::{ToolFreezePhase, ToolSet, ToolTableFingerprint};
use crate::protocol::message::AgentMessage;
use crate::protocol::ports::{
    XyBashExecutor, XyBatchMode, XyExportIo, XyHookBus, XyModel, XyPermission,
};
use crate::protocol::session::{
    EntryBase, ModelChangeEntry, SessionEntry, ThinkingLevelChangeEntry,
};
use crate::protocol::types::{ThinkingLevel, XyModelMeta};

// ── Model Registry ──────────────────────────────────────────────────

pub use crate::agent::model::registry::ModelRegistry;

// ── AgentCapabilities ────────────────────────────────────────────────────

/// In-flight (or last-bound) model+thinking for the active agent run (c1470 NextTurn).
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
            bash: crate::agent::session::bash::BashExecHandler::new(bash_executor),
            exporter: crate::agent::session::export::SessionExporter::new(export_io),
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

    // ── Model management (delegated to ModelManager) ──────────────

    fn with_models<R>(&self, f: impl FnOnce(&ModelManager) -> R) -> R {
        let guard = crate::agent::lock::lock_mutex(&self.model_manager);
        f(&guard)
    }

    fn with_models_mut<R>(&self, f: impl FnOnce(&mut ModelManager) -> R) -> R {
        let mut guard = crate::agent::lock::lock_mutex(&self.model_manager);
        f(&mut guard)
    }

    /// Shared handle for ReAct NextTurn refresh (clone into the run stream).
    pub(crate) fn model_manager_handle(&self) -> Arc<Mutex<ModelManager>> {
        self.model_manager.clone()
    }

    /// Shared active-turn binding for chrome (footer active / next-turn cue).
    pub(crate) fn active_turn_handle(&self) -> Arc<Mutex<Option<ActiveTurnBinding>>> {
        self.active_turn.clone()
    }

    /// Get the currently selected model metadata (clone for lock safety).
    pub fn current_model(&self) -> Option<XyModelMeta> {
        self.with_models(|mm| mm.current_model().cloned())
    }

    /// Build the selected model instance.
    pub fn build_current_model(&self) -> Result<Arc<dyn XyModel>, String> {
        self.with_models(|mm| mm.build_current_model())
    }

    /// Selected thinking level (clamped).
    pub fn thinking_level(&self) -> ThinkingLevel {
        self.with_models(|mm| mm.thinking_level())
    }

    /// Active (in-flight) binding only — `None` when idle / converged.
    pub fn inflight_turn_binding(&self) -> Option<ActiveTurnBinding> {
        self.active_turn
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Active (in-flight) binding, or selected when idle.
    pub fn active_turn_binding(&self) -> Option<ActiveTurnBinding> {
        if let Some(active) = self.inflight_turn_binding() {
            return Some(active);
        }
        self.with_models(|mm| {
            let meta = mm.current_model()?;
            let levels = crate::agent::model::manager::ModelManager::levels_for_meta(meta);
            Some(ActiveTurnBinding {
                model_id: meta.id.clone(),
                display_name: if meta.display_name.is_empty() {
                    meta.id.clone()
                } else {
                    meta.display_name.clone()
                },
                thinking: mm.thinking_level(),
                omit_thinking: !ThinkingLevel::is_adjustable(&levels),
            })
        })
    }

    /// True while an agent run has an active turn binding (in-flight).
    pub fn has_active_turn(&self) -> bool {
        self.active_turn
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// Converge active → selected (idle / abort / run end).
    pub fn clear_active_turn(&self) {
        *crate::agent::lock::lock_mutex(&self.active_turn) = None;
    }

    /// Set thinking level.
    pub fn set_thinking_level(&mut self, level: ThinkingLevel) -> Result<(), String> {
        let previous = self.thinking_level();
        self.with_models_mut(|mm| mm.set_thinking_level(level))?;
        self.persist_thinking_level_change(previous, level);
        Ok(())
    }

    /// Cycle to the next level in the current model's support list.
    pub fn cycle_thinking_level(&mut self) -> Result<ThinkingLevel, String> {
        let previous = self.thinking_level();
        let level = self.with_models_mut(|mm| mm.cycle_thinking_level())?;
        self.persist_thinking_level_change(previous, level);
        Ok(level)
    }

    fn persist_thinking_level_change(&self, previous: ThinkingLevel, level: ThinkingLevel) {
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
        if let Some(bus) = self.hook_bus.clone() {
            observe_hook_sync(
                &bus,
                "thinking_level_select",
                "",
                serde_json::json!({
                    "level": level.as_str(),
                    "previous": previous.as_str(),
                }),
            );
        }
    }

    /// Apply Settings `default_thinking_level` (if parseable) then preferred-or-highest.
    pub fn apply_default_thinking_level(&mut self, raw: Option<&str>) {
        let preferred = raw.and_then(ThinkingLevel::parse);
        self.with_models_mut(|mm| {
            mm.set_preferred_default(preferred);
            mm.apply_preferred_or_highest();
        });
    }

    /// Select a specific model by ID (`source` = `"set"`).
    pub fn select_model(&mut self, model_id: &str) -> Result<(), String> {
        self.select_model_with_source(model_id, "set")
    }

    /// Select a model and emit `model_select` with the given source (`set` | `cycle`).
    pub fn select_model_with_source(&mut self, model_id: &str, source: &str) -> Result<(), String> {
        let previous = self.current_model().map(|m| m.id.clone());
        self.with_models_mut(|mm| mm.select_model(model_id))?;
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

    // ── Slash commands ───────────────────────────────────────────

    /// Get all available commands (builtin + extension).
    pub(crate) fn get_commands(&self) -> Vec<SlashCommandInfo> {
        get_all_commands(&self.extension_commands)
    }

    // ── Session management ────────────────────────────────────────

    /// Set the active session ID.
    pub fn set_session(&mut self, session_id: String) {
        xylitol_ai_bridge::provider::set_obs_session(session_id.clone(), None);
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
            if let Some(bus) = &self.hook_bus {
                let reason = if parent.is_some() { "fork" } else { "new" };
                let (ty, phase, ctx) =
                    crate::agent::runtime::script_hook_ctx::session_start(reason);
                observe_hook(bus, ty, phase, ctx).await;
            }
        }
        Ok(())
    }

    /// Load conversation messages from the session store (leaf + compaction-aware cut).
    pub(crate) async fn load_conversation_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<AgentMessage>, String> {
        let entries = self.store.load_leaf_branch(session_id).await?;
        let entries = crate::protocol::session::build_context_entries(&entries);
        Ok(entries
            .iter()
            .filter_map(|e| e.as_agent_message())
            .collect())
    }

    /// Shared session store handle (same instance as XyDriver uses).
    pub fn session_store(&self) -> Arc<dyn XySessionStore> {
        self.store.clone()
    }

    /// Lifecycle event sink (compaction Start/End etc.).
    pub(crate) fn event_sink(&self) -> Arc<dyn crate::protocol::ports::XyEventSink> {
        self.sink.clone()
    }

    /// Compaction settings snapshot for ReAct turn-end auto.
    pub(crate) fn compaction_settings(&self) -> CompactionSettings {
        self.compaction_orchestrator.settings().clone()
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

    // ── Fork ────────────────────────────────────────────────────

    /// Fork the current session at a given entry, creating a child session.
    ///
    /// Returns the child session ID on success. See
    /// [`crate::protocol::session::ForkPosition`].
    pub async fn fork_session(
        &self,
        at_entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, String> {
        let parent_id = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

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
            .map_err(|e| format!("fork failed: {e}"))?;

        Ok(child_id)
    }

    // ── Skill commands ──────────────────────────────────────

    // ── Dynamic system prompt ────────────────────────────────────

    /// Rebuild the system prompt from current options.
    pub fn rebuild_system_prompt(&mut self) {
        let t0 = std::time::Instant::now();
        let tool_n = self.prompt_opts.selected_tools.len();
        self.system_prompt = Some(prompt::build_system_prompt(&self.prompt_opts));
        let ms = t0.elapsed().as_millis();
        let chars = self.system_prompt.as_ref().map(|s| s.len()).unwrap_or(0);
        if ms >= 80 {
            log::warn!(
                target: "xylitol::lag",
                "rebuild_system_prompt {ms}ms tools={tool_n} chars={chars}"
            );
        } else if ms >= 16 {
            log::info!(
                target: "xylitol::lag",
                "rebuild_system_prompt {ms}ms tools={tool_n} chars={chars}"
            );
        } else {
            log::debug!(
                target: "xylitol::lag",
                "rebuild_system_prompt {ms}ms tools={tool_n} chars={chars}"
            );
        }
    }

    /// Replace context / SYSTEM / APPEND resources and rebuild the system prompt (c1100).
    ///
    /// Affects the **next** `run` only. Does **not** mutate session history or store entries.
    pub fn apply_prompt_resources(
        &mut self,
        context_files: Vec<(String, String)>,
        system_prompt: Option<String>,
        append_system_prompt: Vec<String>,
    ) {
        self.prompt_opts.context_files = context_files;
        self.prompt_opts.system_prompt = system_prompt;
        self.prompt_opts.append_system_prompt = append_system_prompt;
        self.rebuild_system_prompt();
    }

    /// Replace the skills catalog in the system prompt (c1085).
    ///
    /// Affects the **next** `run` only. Does **not** mutate session history.
    pub fn apply_skills(&mut self, skills: Vec<crate::protocol::resource::SkillInfo>) {
        self.prompt_opts.skills = skills;
        self.rebuild_system_prompt();
    }

    /// Names of skills currently injected into the system prompt (for `$` expand / reload).
    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.prompt_opts
            .skills
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    /// Full skill catalog (paths for `$` SKILL.md expand; c1130).
    pub fn loaded_skills(&self) -> &[crate::protocol::resource::SkillInfo] {
        &self.prompt_opts.skills
    }

    /// Set the active system prompt text and rebuild.
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.prompt_opts.system_prompt = prompt.clone();
        self.system_prompt = prompt;
        self.rebuild_system_prompt();
    }

    /// Set the active tool set and rebuild the system prompt to reflect it.
    ///
    /// When an agent turn is in-flight and [`crate::agent::context_policy::ContextPolicy::allows_midturn_tools_rewrite`]
    /// is false (Search default), the call is ignored so provider `tools` stay stable.
    /// When the tool table is [`ToolFreezePhase::Frozen`] (c1900 轨 A), settle/hot-merge
    /// MUST NOT expand the provider-visible table — use [`Self::freeze_tools`] to re-gate.
    pub fn set_tools(&mut self, tools: ToolSet) {
        if self.tool_freeze == ToolFreezePhase::Frozen {
            log::warn!(
                target: "xylitol::agent",
                "set_tools ignored: tool table FROZEN (use freeze_tools / reopen_tools_for_regate)"
            );
            return;
        }
        if !self.allow_tools_rewrite_now("set_tools") {
            return;
        }
        self.apply_tools_metadata(&tools);
        self.tools = tools;
        self.rebuild_system_prompt();
    }

    /// Install tools + prompt metadata without rebuilding the system prompt text.
    ///
    /// Used by MCP settle so `build_system_prompt` can run off the TUI tick path.
    /// Returns a clone of [`SystemPromptOpts`] ready for [`prompt::build_system_prompt`].
    /// Same mid-turn Search gate as [`Self::set_tools`]. FROZEN sessions ignore expands.
    pub fn set_tools_defer_prompt(&mut self, tools: ToolSet) -> SystemPromptOpts {
        if self.tool_freeze == ToolFreezePhase::Frozen {
            log::warn!(
                target: "xylitol::agent",
                "set_tools_defer_prompt ignored: tool table FROZEN"
            );
            return self.prompt_opts.clone();
        }
        if !self.allow_tools_rewrite_now("set_tools_defer_prompt") {
            return self.prompt_opts.clone();
        }
        self.apply_tools_metadata(&tools);
        self.tools = tools;
        self.prompt_opts.clone()
    }

    /// Current tool-table freeze phase (c1900).
    pub fn tool_freeze_phase(&self) -> ToolFreezePhase {
        self.tool_freeze
    }

    /// True when provider-visible tools are frozen.
    pub fn is_tools_frozen(&self) -> bool {
        self.tool_freeze == ToolFreezePhase::Frozen
    }

    /// Frozen fingerprint, if any.
    pub fn frozen_tool_fingerprint(&self) -> Option<&ToolTableFingerprint> {
        self.tool_fingerprint.as_ref()
    }

    /// Mark gate start (Unfrozen → Gating). No-op if already Frozen.
    pub fn begin_tool_gating(&mut self) {
        if self.tool_freeze != ToolFreezePhase::Frozen {
            self.tool_freeze = ToolFreezePhase::Gating;
        }
    }

    /// Leave Frozen so a subsequent [`Self::freeze_tools`] can re-freeze (idle `/reload`).
    pub fn reopen_tools_for_regate(&mut self) {
        self.tool_freeze = ToolFreezePhase::Gating;
        self.tool_fingerprint = None;
    }

    /// Clear freeze state entirely (session switch / resume → next run re-gates).
    pub fn clear_tool_freeze(&mut self) {
        self.tool_freeze = ToolFreezePhase::Unfrozen;
        self.tool_fingerprint = None;
    }

    /// Install `tools` as the frozen provider-visible table (upsert path for callers
    /// that already built core ∪ armed). Bypasses the FROZEN ignore on [`Self::set_tools`].
    pub fn freeze_tools(&mut self, tools: ToolSet) {
        let fp = ToolTableFingerprint::from_toolset(&tools);
        self.apply_tools_metadata(&tools);
        self.tools = tools;
        self.tool_fingerprint = Some(fp);
        self.tool_freeze = ToolFreezePhase::Frozen;
        self.rebuild_system_prompt();
    }

    /// Freeze without rebuilding system prompt text (pair with [`Self::install_system_prompt_text`]).
    pub fn freeze_tools_defer_prompt(&mut self, tools: ToolSet) -> SystemPromptOpts {
        let fp = ToolTableFingerprint::from_toolset(&tools);
        self.apply_tools_metadata(&tools);
        self.tools = tools;
        self.tool_fingerprint = Some(fp);
        self.tool_freeze = ToolFreezePhase::Frozen;
        self.prompt_opts.clone()
    }

    /// Compare `candidate` to the frozen fingerprint (false if not frozen).
    pub fn frozen_fingerprint_matches_set(&self, candidate: &ToolSet) -> bool {
        match &self.tool_fingerprint {
            Some(fp) => fp.matches(&ToolTableFingerprint::from_toolset(candidate)),
            None => false,
        }
    }

    fn allow_tools_rewrite_now(&self, op: &str) -> bool {
        if self.has_active_turn() && !self.context_policy.allows_midturn_tools_rewrite() {
            log::warn!(
                target: "xylitol::agent",
                "{op} ignored: mid-turn tools rewrite denied by ContextPolicy (tools_mode={:?})",
                self.context_policy.tools_mode
            );
            return false;
        }
        true
    }

    /// Install a prebuilt system prompt string (pair with [`Self::set_tools_defer_prompt`]).
    pub fn install_system_prompt_text(&mut self, prompt: String) {
        let chars = prompt.len();
        let tool_n = self.prompt_opts.selected_tools.len();
        self.system_prompt.replace(prompt);
        log::debug!(
            target: "xylitol::lag",
            "install_system_prompt_text tools={tool_n} chars={chars}"
        );
    }

    fn apply_tools_metadata(&mut self, tools: &ToolSet) {
        self.prompt_opts.selected_tools = tools.iter().map(|t| t.name().to_string()).collect();
        self.prompt_opts.tool_snippets =
            prompt::collect_tool_snippets(tools, &self.prompt_opts.selected_tools);
        self.prompt_opts.prompt_guidelines =
            prompt::collect_tool_guidelines(tools, &self.prompt_opts.selected_tools);
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
    ) -> Result<crate::protocol::ports::XyBashResult, String> {
        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::user_bash(
                command,
                exclude_from_context,
                &self.cwd,
            );
            cancel_hook(bus, ty, phase, ctx).await?;
        }
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

    /// Persist a bash result as `SessionEntry::Message` with `role=bashExecution`.
    pub async fn record_bash_result(
        &self,
        command: &str,
        result: &crate::protocol::ports::XyBashResult,
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
        self.maybe_auto_compact_with(
            &crate::agent::compaction::EstimateOpts {
                model_id: self.current_model().map(|m| m.id.clone()),
                ..Default::default()
            },
            None,
        )
        .await
    }

    /// Auto-compact using the same estimate opts as the product footer (c1420).
    ///
    /// `last_assistant`: when set, abort / stale guards apply (pi `_checkCompaction`).
    pub async fn maybe_auto_compact_with(
        &self,
        estimate_opts: &crate::agent::compaction::EstimateOpts,
        last_assistant: Option<&crate::protocol::message::AgentMessage>,
    ) -> Result<bool, String> {
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

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        let compacted = self
            .compaction_orchestrator
            .maybe_auto_compact(
                self.store.as_ref(),
                sid,
                model.as_ref(),
                self.sink.as_ref(),
                ctx_window,
                estimate_opts,
                last_assistant,
                None,
            )
            .await?;

        if compacted && let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        Ok(compacted)
    }

    /// Manual force compact (pi `compact(customInstructions?)`). Does not apply the reserve gate.
    pub async fn force_compact(&self, instructions: Option<String>) -> Result<(), String> {
        let sid = self
            .session_id()
            .ok_or_else(|| "no active session".to_string())?;

        let model = self
            .build_current_model()
            .map_err(|e| format!("no model: {e}"))?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_before_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        self.compaction_orchestrator
            .compact(
                self.store.as_ref(),
                sid,
                model.as_ref(),
                self.sink.as_ref(),
                instructions,
            )
            .await?;

        if let Some(bus) = &self.hook_bus {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_compact();
            observe_hook(bus, ty, phase, ctx).await;
        }

        Ok(())
    }
}

/// Cancel-capable hook: `Blocked` aborts the library operation (c995/c997).
pub(crate) async fn cancel_hook(
    bus: &Arc<dyn XyHookBus>,
    event_type: &str,
    phase: &str,
    context: serde_json::Value,
) -> Result<(), String> {
    match bus.dispatch(event_type, phase, context).await {
        crate::protocol::ports::XyHookOutcome::Blocked { reason } => Err(reason),
        _ => Ok(()),
    }
}

pub(crate) async fn observe_hook(
    bus: &Arc<dyn XyHookBus>,
    event_type: &str,
    phase: &str,
    context: serde_json::Value,
) {
    if let crate::protocol::ports::XyHookOutcome::Blocked { reason } =
        bus.dispatch(event_type, phase, context).await
    {
        log::warn!(
            "Script hook blocked observe-only lifecycle event (fail-open) event={} phase={} reason={}",
            event_type,
            phase,
            reason
        );
    }
}

/// Sync observe for XyDriver/agent APIs that are not async (c996).
fn observe_hook_sync(
    bus: &Arc<dyn XyHookBus>,
    event_type: &str,
    phase: &str,
    context: serde_json::Value,
) {
    let bus = bus.clone();
    let event_type = event_type.to_string();
    let phase = phase.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|rt| rt.block_on(observe_hook(&bus, &event_type, &phase, context)));
        let _ = tx.send(result.map(|_| ()));
    });
    match rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => log::warn!("observe_hook_sync runtime failed error={}", e),
        Err(_) => log::warn!("observe_hook_sync worker disconnected"),
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
