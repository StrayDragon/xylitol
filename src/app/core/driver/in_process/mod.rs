//! In-process [`XyInProcessDriver`].

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agent::AgentRuntime;
use crate::agent::runtime::RunPolicy;
use crate::app::core::bang_exec::BangExecHandler;
use crate::app::core::session_export::SessionExporter;
use crate::protocol::error::XySessionError;
use crate::protocol::ports::{XyBashResult, XySessionStore};
use crate::protocol::session::{SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel};

use super::types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    ModelInfo, ProjectTrustMode, ProjectTrustPersistReport, RuntimeReloadReport, SessionListEntry,
    SessionStats,
};
pub(super) use super::{XyDriver, XyDriverError, types};

mod mcp;
mod reload;
mod session;
#[cfg(test)]
mod tests;

use mcp::{McpBootState, McpDiscoverOk};

// ── In-process driver ─────────────────────────────────────────────

struct InProcessReloadState {
    cwd: std::path::PathBuf,
    agent_dir: std::path::PathBuf,
    project_trusted: bool,
    mcp: crate::app::core::composition::McpSession,
    mcp_servers: Vec<crate::app::core::mcp_spec::McpServerSpec>,
}

/// In-process driver wrapping the local agent module.
///
/// Constructed at the composition root (`app::cli` via `bootstrap`) which wires
/// ports and agent together. This is the **only** place in the app surfaces
/// that imports `agent`.
pub struct XyInProcessDriver {
    agent: AgentRuntime,
    /// Session store, held so SwitchSession/GetMessages/Fork can operate. The
    /// agent holds its own clone internally; this one is the surface's handle
    /// for session-management commands.
    store: Arc<dyn XySessionStore>,
    /// Interactive bang (`!` / `!!`) executor — app-surface, not agent.
    bang: BangExecHandler,
    /// Session HTML/JSONL export-import — app-surface.
    exporter: SessionExporter,
    /// Optional reload state for `/reload` and MCP ownership (c1120).
    reload: Option<InProcessReloadState>,
    /// Background MCP bootstrap (c1200); independent of agent busy.
    mcp_boot: McpBootState,
    /// Discover finished after gate timeout detached `Running` (apply on poll).
    late_mcp_discover: Option<tokio::task::JoinHandle<McpDiscoverOk>>,
    /// User-visible notice after gate timeout subset freeze (TUI takes once).
    mcp_gate_notice: Option<String>,
    /// When set, [`Self::poll_mcp_bootstrap`] freezes tools at settle or this deadline (TUI gate).
    tool_gate_deadline: Option<std::time::Instant>,
    /// TUI-only ask gateway; MCP reload MUST re-plus ask when this is set (c1850).
    ask_gateway: Option<Arc<dyn crate::protocol::ports::ask::AskUserGateway>>,
    /// Session-bound Todo SSOT gateway shared by `todo_*` builtins (c1955).
    todo_gateway: Arc<crate::infra::tools::SessionAgentTodoGateway>,
}

impl XyInProcessDriver {
    /// Construct from a built agent plus the store used to build it.
    ///
    /// Installs default bang + export I/O for product surfaces.
    /// Use [`Self::with_surface_ops`] to override.
    pub fn new(agent: AgentRuntime, store: Arc<dyn XySessionStore>) -> Self {
        Self::with_surface_ops(
            agent,
            store,
            BangExecHandler::new(Some(Arc::new(
                crate::infra::bash_exec::InfraBashExecutor::new(),
            ))),
            SessionExporter::new(Some(Arc::new(crate::infra::export::StdExportIo::new()))),
        )
    }

    /// Construct with explicit bang + export collaborators (tests / embed).
    pub fn with_surface_ops(
        agent: AgentRuntime,
        store: Arc<dyn XySessionStore>,
        bang: BangExecHandler,
        exporter: SessionExporter,
    ) -> Self {
        let todo_gateway = crate::infra::tools::SessionAgentTodoGateway::new(store.clone());
        let mut driver = Self {
            agent,
            store,
            bang,
            exporter,
            reload: None,
            mcp_boot: McpBootState::Idle,
            late_mcp_discover: None,
            mcp_gate_notice: None,
            tool_gate_deadline: None,
            ask_gateway: None,
            todo_gateway: todo_gateway.clone(),
        };
        // Replace ephemeral MemoryTodoGateway from build_agent with store-bound SSOT.
        driver.set_tools(crate::agent::tools::ToolSet::from_iter(
            crate::infra::tools::default_tools_with_todo(todo_gateway),
        ));
        driver
    }

    /// Construct with an explicit bang handler (tests / embed without shell).
    pub fn with_bang(
        agent: AgentRuntime,
        store: Arc<dyn XySessionStore>,
        bang: BangExecHandler,
    ) -> Self {
        Self::with_surface_ops(
            agent,
            store,
            bang,
            SessionExporter::new(Some(Arc::new(crate::infra::export::StdExportIo::new()))),
        )
    }

    /// Install TUI-only `ask` tool and remember the gateway for MCP reload.
    pub fn install_ask_tool(
        &mut self,
        gateway: Arc<dyn crate::protocol::ports::ask::AskUserGateway>,
    ) {
        self.ask_gateway = Some(gateway.clone());
        self.set_tools(crate::agent::tools::ToolSet::from_iter(
            crate::infra::tools::default_tools_with_ask_and_todo(
                gateway,
                self.todo_gateway.clone(),
            ),
        ));
    }

    /// Gateway used when rebuilding builtins during MCP settle / reload (c1850).
    pub fn ask_gateway(&self) -> Option<Arc<dyn crate::protocol::ports::ask::AskUserGateway>> {
        self.ask_gateway.clone()
    }

    /// Builtins for ToolSet rebuild: store-bound Todo (+ optional ask).
    pub fn builtins_for_reload(&self) -> Vec<Arc<dyn crate::protocol::ports::XyTool>> {
        match &self.ask_gateway {
            Some(g) => crate::infra::tools::default_tools_with_ask_and_todo(
                g.clone(),
                self.todo_gateway.clone(),
            ),
            None => crate::infra::tools::default_tools_with_todo(self.todo_gateway.clone()),
        }
    }

    async fn bind_todo_session(&self, session_id: Option<&str>) {
        self.todo_gateway
            .bind_session(session_id.map(str::to_string))
            .await;
    }

    /// Store-bound Todo gateway (shared with `todo_*` builtins).
    pub fn todo_gateway(&self) -> Arc<crate::infra::tools::SessionAgentTodoGateway> {
        self.todo_gateway.clone()
    }

    /// Enable `/reload` and MCP ownership for the process lifetime (c1120).
    pub fn enable_reload_state(
        &mut self,
        cwd: std::path::PathBuf,
        agent_dir: std::path::PathBuf,
        project_trusted: bool,
        mcp_servers: Vec<crate::app::core::mcp_spec::McpServerSpec>,
    ) {
        self.reload = Some(InProcessReloadState {
            cwd,
            agent_dir,
            project_trusted,
            mcp: crate::app::core::composition::McpSession::new(),
            mcp_servers,
        });
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.agent.cancel_token()
    }

    /// Replace the tool set (next `run`). Used by composition MCP reload.
    /// Ignored while FROZEN (c1900); prefer [`Self::freeze_tools`].
    pub fn set_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.agent.set_tools(tools);
    }

    /// Freeze provider-visible tools (c1900 轨 A). Used by MCP gate / `/reload` re-freeze.
    pub fn freeze_tools(&mut self, tools: crate::agent::tools::ToolSet) {
        self.agent.freeze_tools(tools);
    }

    pub fn reopen_tools_for_regate(&mut self) {
        self.agent.reopen_tools_for_regate();
    }

    pub fn is_tools_frozen(&self) -> bool {
        self.agent.is_tools_frozen()
    }

    /// Replace context / SYSTEM / APPEND for the next `run` (c1100).
    /// Does not mutate session history.
    pub fn apply_prompt_resources(
        &mut self,
        context_files: Vec<(String, String)>,
        system_prompt: Option<String>,
        append_system_prompt: Vec<String>,
    ) {
        self.agent
            .apply_prompt_resources(context_files, system_prompt, append_system_prompt);
    }

    /// Replace skills catalog for the next `run` (c1085). Does not mutate history.
    pub fn apply_skills(&mut self, skills: Vec<crate::protocol::resource::SkillInfo>) {
        self.agent.apply_skills(skills);
    }

    /// Names currently in the system `<available_skills>` catalog (c1085).
    pub fn loaded_skill_names(&self) -> Vec<String> {
        self.agent.loaded_skill_names()
    }

    /// Full skill catalog for `$` completion / expand (c1130).
    pub fn loaded_skills(&self) -> Vec<crate::protocol::resource::SkillInfo> {
        self.agent.loaded_skills()
    }

    fn skill_catalog_pairs(&self) -> Vec<(String, String)> {
        self.agent
            .loaded_skills()
            .iter()
            .map(|s| (s.name.clone(), s.description.clone().unwrap_or_default()))
            .collect()
    }

    /// Test/diagnostics: tool names currently registered.
    #[cfg(test)]
    pub(crate) fn tool_names_for_test(&self) -> Vec<String> {
        self.agent.tool_names()
    }

    /// Test/diagnostics: assembled system prompt text (c1100).
    #[cfg(test)]
    pub(crate) fn system_prompt_for_test(&self) -> Option<String> {
        self.agent.system_prompt().map(String::from)
    }
}

fn bind_session_or_err(
    agent: &mut AgentRuntime,
    session_id: impl Into<String>,
) -> Result<(), XyDriverError> {
    agent.bind_session(session_id).map_err(XyDriverError::from)
}

fn require_active_session(agent: &AgentRuntime) -> Result<&str, XyDriverError> {
    agent
        .session_id()
        .ok_or_else(|| XySessionError::NoActiveSession.into())
}

#[async_trait]
impl XyDriver for XyInProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        self.ensure_tool_table_frozen().await;
        if self.agent.session_id().is_none() {
            let id = uuid::Uuid::new_v4().to_string();
            let _ = bind_session_or_err(&mut self.agent, id.clone());
            self.bind_todo_session(Some(&id)).await;
        } else if let Some(sid) = self.agent.session_id() {
            // Keep Todo gateway aligned if agent was bound before driver wiring.
            self.bind_todo_session(Some(sid)).await;
        }
        let stream = self.agent.submit_root(prompt, RunPolicy::Reject).await;
        Box::pin(stream)
    }

    fn abort(&self) {
        self.bang.abort();
        self.agent.abort();
    }

    fn current_model(&self) -> Option<ModelInfo> {
        self.agent.current_model().map(|m| ModelInfo::from(&m))
    }

    fn active_turn(&self) -> Option<(String, String, bool)> {
        let binding = self.agent.inflight_turn_binding()?;
        Some((
            binding.display_name,
            binding.thinking,
            binding.omit_thinking,
        ))
    }

    fn has_active_turn(&self) -> bool {
        self.agent.has_active_turn()
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        self.agent
            .model_registry()
            .list()
            .iter()
            .map(ModelInfo::from)
            .collect()
    }

    async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        // Prefer registry id; only use unique upstream `config.model` as alias.
        let registry = self.agent.model_registry();
        let found = registry
            .list()
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.id.clone())
            .or_else(|| {
                let hits: Vec<_> = registry
                    .list()
                    .iter()
                    .filter(|m| m.config.model == model_id)
                    .collect();
                match hits.as_slice() {
                    [only] => Some(only.id.clone()),
                    _ => None,
                }
            })
            .ok_or_else(|| XyDriverError::not_found(format!("model not found: {model_id}")))?;
        self.agent
            .select_model(&found)
            .await
            .map_err(XyDriverError::from)?;
        // Re-read the resolved model to return authoritative info.
        Ok(self
            .agent
            .current_model()
            .map(|m| ModelInfo::from(&m))
            .unwrap_or_else(|| ModelInfo {
                id: found.clone(),
                display_name: found,
                thinking: true,
                thinking_levels: Vec::new(),
                context_window: 0,
            }))
    }

    async fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
        let list = self.agent.model_registry().list().to_vec();
        if list.is_empty() {
            return Err(XyDriverError::not_found("no models available"));
        }
        let current_id = self.agent.current_model().map(|m| m.id.clone());
        let current_idx = current_id
            .as_ref()
            .and_then(|cur| list.iter().position(|m| m.id == *cur))
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % list.len();
        let next_id = list[next_idx].id.clone();
        self.agent
            .select_model_with_source(&next_id, "cycle")
            .await
            .map_err(XyDriverError::from)?;
        Ok(ModelInfo::from(&list[next_idx]))
    }

    async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        self.agent
            .set_thinking_level(level)
            .await
            .map_err(XyDriverError::from)
    }

    fn thinking_level(&self) -> String {
        self.agent.thinking_level()
    }

    async fn cycle_thinking_level(&mut self) -> Result<String, XyDriverError> {
        self.agent
            .cycle_thinking_level()
            .await
            .map_err(XyDriverError::from)
    }

    fn session_id(&self) -> Option<String> {
        self.agent.session_id().map(String::from)
    }

    async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
        chunk_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
    ) -> Result<XyBashResult, XyDriverError> {
        if let Some(bus) = self.agent.hook_bus() {
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::user_bash(
                command,
                exclude_from_context,
                self.agent.cwd(),
            );
            crate::agent::capabilities::cancel_hook(&bus, ty, phase, ctx).await?;
        }
        self.bang
            .execute(
                self.store.as_ref(),
                self.agent.session_id(),
                command,
                exclude_from_context,
                chunk_tx,
            )
            .await
            .map_err(XyDriverError::from)
    }

    async fn compact(&mut self, instructions: Option<String>) -> Result<bool, XyDriverError> {
        // Force path (c1640 / pi compact) — MUST NOT use maybe_auto_compact.
        self.agent
            .force_compact(instructions)
            .await
            .map(|()| true)
            .map_err(XyDriverError::from)
    }

    async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = require_active_session(&self.agent)?.to_string();
        self.exporter
            .export_to_html(self.store.as_ref(), &sid, path)
            .await?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = require_active_session(&self.agent)?.to_string();
        self.exporter
            .export_to_jsonl(self.store.as_ref(), &sid, path)
            .await?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        self.exporter
            .import_from_jsonl(self.store.as_ref(), path)
            .await
    }

    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        XyInProcessDriver::fork_session(self, entry_id, position).await
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        XyInProcessDriver::switch_session(self, session_id).await
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        XyInProcessDriver::get_messages(self).await
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        XyInProcessDriver::get_session_stats(self).await
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::model::ContextTokenEstimate, XyDriverError> {
        XyInProcessDriver::estimate_context_tokens(self).await
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        crate::app::product_commands::product_slash_commands()
            .into_iter()
            .map(|c| CommandInfo {
                name: c.name.to_string(),
                description: c.description.to_string(),
            })
            .collect()
    }

    fn steer(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.agent.steer(message);
        Ok(())
    }

    fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.agent.follow_up(message);
        Ok(())
    }

    fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        self.agent.clear_queues(clear_steer, clear_follow_up);
        Ok(())
    }

    fn queue_stats(&self) -> crate::agent::capabilities::QueueStats {
        self.agent.queue_stats()
    }

    async fn session_tree(
        &self,
        kind: SessionTreeKind,
    ) -> Result<Vec<SessionTreeNode>, XyDriverError> {
        XyInProcessDriver::session_tree(self, kind).await
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        XyInProcessDriver::travel_session_tree(self, kind, entry_id).await
    }

    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        XyInProcessDriver::append_entry_label(self, target_id, label).await
    }

    fn leaf_entry_id(&self) -> Option<String> {
        XyInProcessDriver::leaf_entry_id(self)
    }

    async fn load_debug_scene(&mut self, scene: &str) -> Result<DebugSceneLoad, XyDriverError> {
        XyInProcessDriver::load_debug_scene(self, scene).await
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        XyInProcessDriver::list_sessions(self).await
    }

    async fn load_session_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        XyInProcessDriver::load_session_entries(self, session_id).await
    }

    async fn new_session(&mut self) -> Result<String, XyDriverError> {
        XyInProcessDriver::new_session(self).await
    }

    async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        XyInProcessDriver::get_session_name(self).await
    }

    async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
        XyInProcessDriver::set_session_name(self, name).await
    }

    async fn set_session_name_for(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XyDriverError> {
        XyInProcessDriver::set_session_name_for(self, session_id, name).await
    }

    async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        XyInProcessDriver::delete_session(self, session_id).await
    }

    fn session_store(&self) -> Option<Arc<dyn XySessionStore>> {
        Some(self.store.clone())
    }

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        self.skill_catalog_pairs()
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        XyInProcessDriver::loaded_resources_snapshot(self).await
    }

    fn mcp_blocks_agent(&self) -> bool {
        XyInProcessDriver::mcp_blocks_agent(self)
    }

    fn is_tools_frozen(&self) -> bool {
        self.agent.is_tools_frozen()
    }

    async fn arm_tool_freeze_gate(&mut self) {
        self.arm_tool_freeze_gate_inner().await
    }

    fn take_mcp_gate_notice(&mut self) -> Option<String> {
        self.take_mcp_gate_notice_inner()
    }

    async fn begin_mcp_bootstrap(&mut self) {
        XyInProcessDriver::begin_mcp_bootstrap(self).await
    }

    async fn poll_mcp_bootstrap(&mut self) -> bool {
        XyInProcessDriver::poll_mcp_bootstrap(self).await
    }

    async fn reload_runtime(
        &mut self,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        XyInProcessDriver::reload_runtime(self, cancel).await
    }

    fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        XyInProcessDriver::persist_project_trust(self, mode)
    }

    async fn copy_text_to_clipboard(
        &mut self,
        text: &str,
    ) -> Result<ClipboardCopyOutcome, XyDriverError> {
        let plan = crate::infra::clipboard::plan_clipboard_copy_async(text.to_string())
            .await
            .map_err(XyDriverError::from)?;
        if !plan.will_succeed() {
            return Err(XyDriverError::io(plan.failure_message()));
        }
        Ok(ClipboardCopyOutcome {
            pending_osc52: plan.osc52_sequence,
        })
    }

    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, XyDriverError> {
        let image = tokio::task::spawn_blocking(crate::infra::clipboard::read_clipboard_image)
            .await
            .map_err(|e| XyDriverError::io(format!("clipboard image task failed: {e}")))?
            .map_err(XyDriverError::from)?;
        let Some(image) = image else {
            return Ok(None);
        };
        let path = tokio::task::spawn_blocking(move || {
            crate::infra::clipboard::write_clipboard_image_temp(&image.bytes, &image.mime_type)
        })
        .await
        .map_err(|e| XyDriverError::io(format!("clipboard image write task failed: {e}")))?
        .map_err(XyDriverError::from)?;
        Ok(Some(path))
    }

    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        tokio::task::spawn_blocking(crate::infra::clipboard::read_clipboard_text)
            .await
            .map_err(|e| XyDriverError::io(format!("clipboard text task failed: {e}")))?
            .map_err(XyDriverError::from)
    }
}
