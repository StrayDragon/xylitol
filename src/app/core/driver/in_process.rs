//! In-process [`XyInProcessDriver`].

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::agent::AgentRuntime;
use crate::agent::runtime::RunPolicy;
use crate::app::core::bang_exec::BangExecHandler;
use crate::app::core::session_export::SessionExporter;
use crate::protocol::ports::{XyBashResult, XySessionStore};
use crate::protocol::session::{
    SessionEntry, SessionTreeKind, SessionTreeNode, SessionTreeTravel, plan_message_history_travel,
};

use super::XyDriver;
use super::XyDriverError;
use super::types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    ModelInfo, ProjectTrustMode, ProjectTrustPersistReport, ReloadStepReport, RuntimeReloadReport,
    SessionListEntry, SessionStats, estimate_from_session_entries, session_tree_kind_unimplemented,
    tokenizer_override_from_app_config,
};

// ── In-process driver ─────────────────────────────────────────────

struct InProcessReloadState {
    cwd: std::path::PathBuf,
    agent_dir: std::path::PathBuf,
    project_trusted: bool,
    mcp: crate::app::core::composition::McpSession,
    mcp_servers: Vec<crate::app::core::mcp_spec::McpServerSpec>,
}

type McpToolList = Vec<Arc<dyn crate::protocol::ports::XyTool>>;
type McpDiscoverOk = Option<(Arc<crate::infra::mcp::McpClientManager>, McpToolList)>;
type McpDiscoverOutcome = Result<McpDiscoverOk, String>;

/// Whether connecting progress should invalidate the TUI loaded-resources strip.
///
/// `last_ui_label`: `None` = never published; `Some(label)` = last published value.
fn mcp_progress_needs_ui_refresh(
    last_ui_label: &mut Option<Option<String>>,
    current: Option<String>,
) -> bool {
    if last_ui_label.as_ref() == Some(&current) {
        return false;
    }
    *last_ui_label = Some(current);
    true
}

enum McpBootState {
    Idle,
    Running {
        handle: tokio::task::JoinHandle<McpDiscoverOutcome>,
        progress: Arc<tokio::sync::Mutex<crate::infra::mcp::McpConnectProgress>>,
        /// Last connecting label published to the TUI; skip refresh when unchanged.
        last_ui_label: Option<Option<String>>,
    },
    /// Rebuild ToolSet off the TUI tick (default_tools + MCP merge can hitch).
    Rebuilding {
        handle: tokio::task::JoinHandle<crate::agent::tools::ToolSet>,
        manager: Arc<crate::infra::mcp::McpClientManager>,
        tool_n: usize,
        settle_started: std::time::Instant,
    },
    /// Tools already applied; system prompt text building off the tick path.
    Settling {
        handle: tokio::task::JoinHandle<String>,
    },
    Settled,
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
    late_mcp_discover: Option<tokio::task::JoinHandle<McpDiscoverOutcome>>,
    /// User-visible notice after gate timeout subset freeze (TUI takes once).
    mcp_gate_notice: Option<String>,
    /// When set, [`Self::poll_mcp_bootstrap`] freezes tools at settle or this deadline (TUI gate).
    tool_gate_deadline: Option<std::time::Instant>,
    /// TUI-only ask gateway; MCP reload MUST re-plus ask when this is set (c1850).
    ask_gateway: Option<Arc<dyn crate::protocol::ports::ask::AskUserGateway>>,
}

impl XyInProcessDriver {
    fn map_str<T>(r: Result<T, String>) -> Result<T, XyDriverError> {
        r.map_err(XyDriverError::from_opaque)
    }

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
        Self {
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
        }
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
            crate::infra::tools::default_tools_with_ask(gateway),
        ));
    }

    /// Gateway used when rebuilding builtins during MCP settle / reload (c1850).
    pub fn ask_gateway(&self) -> Option<Arc<dyn crate::protocol::ports::ask::AskUserGateway>> {
        self.ask_gateway.clone()
    }

    /// Builtins for ToolSet rebuild: `default_tools` or `default_tools`+ask.
    pub fn builtins_for_reload(&self) -> Vec<Arc<dyn crate::protocol::ports::XyTool>> {
        match &self.ask_gateway {
            Some(g) => crate::infra::tools::default_tools_with_ask(g.clone()),
            None => crate::infra::tools::default_tools(),
        }
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

    /// Initial MCP bootstrap after assembly (cli / server).
    ///
    /// Prefer [`XyDriver::begin_mcp_bootstrap`] + poll for TUI (c1200). This
    /// still performs a blocking reload for callers that need a settled ToolSet
    /// synchronously (legacy / tests).
    pub async fn bootstrap_mcp(&mut self) -> Result<(), XyDriverError> {
        let servers = self
            .reload
            .as_ref()
            .map(|s| s.mcp_servers.clone())
            .unwrap_or_default();
        self.reload_mcp_with_servers(&servers).await?;
        self.mcp_boot = McpBootState::Settled;
        Ok(())
    }

    /// One-line MCP status for startup logs (empty when reload/MCP disabled).
    pub async fn mcp_status_summary(&self) -> Option<String> {
        let state = self.reload.as_ref()?;
        if state.mcp_servers.is_empty() {
            return None;
        }
        let connected = state.mcp.connected_servers().await;
        let diags = state.mcp.diagnostics().await;
        let ids: Vec<_> = connected.iter().map(|s| s.id.as_str()).collect();
        let mut line = format!(
            "MCP: {} configured, {} connected [{}]",
            state.mcp_servers.len(),
            connected.len(),
            ids.join(", ")
        );
        if !diags.is_empty() {
            let detail = diags
                .iter()
                .map(|d| format!("{}: {}", d.server, d.message))
                .collect::<Vec<_>>()
                .join("; ");
            line.push_str(&format!("; diagnostics: {detail}"));
        }
        Some(line)
    }

    async fn reload_mcp_with_servers(
        &mut self,
        servers: &[crate::app::core::mcp_spec::McpServerSpec],
    ) -> Result<(), XyDriverError> {
        let Some(mut state) = self.reload.take() else {
            return Ok(());
        };
        let result = state.mcp.reload(self, servers).await;
        self.reload = Some(state);
        result
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

    /// Wait settle/timeout then freeze the current tool table if not already frozen.
    ///
    /// On gate timeout while still `Running`, detach the discover handle so UI leaves
    /// `connecting i/n` immediately; late results apply via [`Self::poll_mcp_bootstrap`].
    async fn ensure_tool_table_frozen(&mut self) {
        use crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT;

        if self.agent.is_tools_frozen() {
            return;
        }
        self.agent.begin_tool_gating();

        if matches!(self.mcp_boot, McpBootState::Idle) {
            if self
                .reload
                .as_ref()
                .is_some_and(|s| !s.mcp_servers.is_empty())
            {
                self.begin_mcp_bootstrap().await;
            } else {
                self.mcp_boot = McpBootState::Settled;
            }
        }

        let deadline = std::time::Instant::now() + MCP_FIRST_TURN_GATE_TIMEOUT;
        while self.mcp_blocks_agent() && std::time::Instant::now() < deadline {
            let _ = self.poll_mcp_bootstrap().await;
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        if matches!(self.mcp_boot, McpBootState::Running { .. }) {
            log::warn!(
                target: "xylitol::mcp",
                "MCP first-turn gate timed out after {:?}; detaching bootstrap and freezing subset",
                MCP_FIRST_TURN_GATE_TIMEOUT
            );
            self.detach_running_bootstrap_after_gate_timeout();
            self.mcp_gate_notice = Some(
                "MCP gate timed out — tools frozen with armed subset (see /mcp). /reload to retry."
                    .into(),
            );
        } else if self.mcp_blocks_agent() {
            // Rebuilding/Settling past deadline: finish promptly (bounded).
            let extra = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while self.mcp_blocks_agent() && std::time::Instant::now() < extra {
                let _ = self.poll_mcp_bootstrap().await;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }

        let tools = self.agent.tools_snapshot();
        self.agent.freeze_tools(tools);
    }

    /// Detach in-flight `Running` discover so UI can show Settled; apply later on poll.
    fn detach_running_bootstrap_after_gate_timeout(&mut self) {
        let prev = std::mem::replace(&mut self.mcp_boot, McpBootState::Settled);
        if let McpBootState::Running { handle, .. } = prev
            && let Some(old) = self.late_mcp_discover.replace(handle)
        {
            old.abort();
        }
    }

    /// Take one-shot gate timeout notice for TUI chrome / scroll.
    fn take_mcp_gate_notice_inner(&mut self) -> Option<String> {
        self.mcp_gate_notice.take()
    }

    /// Arm first-turn / re-gate freeze without blocking the host loop (TUI).
    ///
    /// [`Self::poll_mcp_bootstrap`] freezes at settle or [`crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT`].
    async fn arm_tool_freeze_gate_inner(&mut self) {
        use crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT;
        if self.agent.is_tools_frozen() {
            return;
        }
        self.agent.begin_tool_gating();
        if self.tool_gate_deadline.is_none() {
            self.tool_gate_deadline = Some(std::time::Instant::now() + MCP_FIRST_TURN_GATE_TIMEOUT);
        }
        if matches!(self.mcp_boot, McpBootState::Idle) {
            if self
                .reload
                .as_ref()
                .is_some_and(|s| !s.mcp_servers.is_empty())
            {
                self.begin_mcp_bootstrap().await;
            } else {
                self.mcp_boot = McpBootState::Settled;
                let _ = self.try_complete_armed_tool_gate();
            }
        } else if matches!(self.mcp_boot, McpBootState::Settled) {
            let _ = self.try_complete_armed_tool_gate();
        }
    }

    fn try_complete_armed_tool_gate(&mut self) -> bool {
        if self.agent.is_tools_frozen() {
            self.tool_gate_deadline = None;
            return false;
        }
        let Some(deadline) = self.tool_gate_deadline else {
            return false;
        };
        let timed_out = std::time::Instant::now() >= deadline;
        if self.mcp_blocks_agent() && !timed_out {
            return false;
        }
        if matches!(self.mcp_boot, McpBootState::Running { .. }) && timed_out {
            log::warn!(
                target: "xylitol::mcp",
                "MCP tool gate timed out; detaching bootstrap and freezing subset"
            );
            self.detach_running_bootstrap_after_gate_timeout();
            self.mcp_gate_notice = Some(
                "MCP gate timed out — tools frozen with armed subset (see /mcp). /reload to retry."
                    .into(),
            );
        } else if self.mcp_blocks_agent() {
            return false;
        }
        let tools = self.agent.tools_snapshot();
        self.agent.freeze_tools(tools);
        self.tool_gate_deadline = None;
        true
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
    agent
        .bind_session(session_id)
        .map_err(|e| XyDriverError::from_opaque(e.to_string()))
}

#[async_trait]
impl XyDriver for XyInProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        self.ensure_tool_table_frozen().await;
        if self.agent.session_id().is_none() {
            let id = uuid::Uuid::new_v4().to_string();
            let _ = bind_session_or_err(&mut self.agent, id);
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

    fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
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

    fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
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
            .map_err(XyDriverError::from)?;
        Ok(ModelInfo::from(&list[next_idx]))
    }

    fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        self.agent
            .set_thinking_level(level)
            .map_err(XyDriverError::from)
    }

    fn thinking_level(&self) -> String {
        self.agent.thinking_level()
    }

    fn cycle_thinking_level(&mut self) -> Result<String, XyDriverError> {
        self.agent
            .cycle_thinking_level()
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
            crate::agent::capabilities::cancel_hook(&bus, ty, phase, ctx)
                .await
                .map_err(XyDriverError::from_opaque)?;
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
        let sid = self
            .agent
            .session_id()
            .ok_or_else(|| XyDriverError::from_opaque("no active session"))?
            .to_string();
        self.exporter
            .export_to_html(self.store.as_ref(), &sid, path)
            .await
            .map_err(XyDriverError::from)?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = self
            .agent
            .session_id()
            .ok_or_else(|| XyDriverError::from_opaque("no active session"))?
            .to_string();
        self.exporter
            .export_to_jsonl(self.store.as_ref(), &sid, path)
            .await
            .map_err(XyDriverError::from)?;
        Ok(path.to_string_lossy().into_owned())
    }

    async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        self.exporter
            .import_from_jsonl(self.store.as_ref(), path)
            .await
            .map_err(XyDriverError::from)
    }

    async fn fork_session(
        &mut self,
        entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<String, XyDriverError> {
        self.agent
            .fork_session(entry_id, position)
            .await
            .map_err(XyDriverError::from)
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        if !self.store.exists(session_id).await {
            return Err(XyDriverError::not_found(format!(
                "session not found: {session_id}"
            )));
        }
        if let Some(bus) = self.agent.hook_bus() {
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_switch("resume", session_id);
            crate::agent::capabilities::cancel_hook(&bus, ty, phase, ctx).await?;
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_shutdown_resume(
                session_id,
                self.agent.session_id(),
            );
            crate::agent::capabilities::observe_hook(&bus, ty, phase, ctx).await;
        }
        let context = Self::map_str(self.store.build_session_context(session_id).await)?;
        bind_session_or_err(&mut self.agent, session_id.to_string())?;
        // Restore precisely what the session recorded. Do not validate, clamp, or
        // append a replacement event: a stale vendor level is intentionally sticky
        // until the user changes or cycles it.
        self.agent.restore_thinking_level(context.thinking_level);
        // c1900: resume/switch starts a new tools epoch — next generate re-gates.
        // Fingerprint match/continue-freeze needs persisted fingerprint (same change wave MAY
        // add Custom/header storage); until then correctness prefers re-freeze.
        self.agent.clear_tool_freeze();
        self.mcp_boot = McpBootState::Idle;
        if self
            .reload
            .as_ref()
            .is_some_and(|s| !s.mcp_servers.is_empty())
        {
            self.begin_mcp_bootstrap().await;
        } else {
            self.mcp_boot = McpBootState::Settled;
        }
        if let Ok(Some(name)) = self.store.get_session_name(session_id).await {
            xylitol_ai_bridge::provider::set_obs_session_name(Some(name.as_str()));
        }
        Ok(session_id.to_string())
    }

    async fn get_messages(&self) -> Result<Vec<SessionEntry>, XyDriverError> {
        let sid = self
            .agent
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        Self::map_str(self.store.load_entries(sid).await)
    }

    async fn get_session_stats(&self) -> Result<SessionStats, XyDriverError> {
        self.agent
            .get_session_stats()
            .await
            .map_err(XyDriverError::from)
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<crate::protocol::model::ContextTokenEstimate, XyDriverError> {
        let entries = self.get_messages().await?;
        let model_id = self.current_model().map(|m| m.id);
        let tokenizer_override = model_id
            .as_deref()
            .and_then(tokenizer_override_from_app_config);
        // HF / local encode is CPU-heavy — keep it off the async worker (TUI host loop).
        tokio::task::spawn_blocking(move || {
            estimate_from_session_entries(&entries, model_id, tokenizer_override)
        })
        .await
        .map_err(|e| XyDriverError::io(format!("estimate join: {e}")))
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
        let sid = self
            .agent
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        if let Some(bus) = self.agent.hook_bus() {
            let kind = format!("{kind:?}");
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_tree(&kind);
            crate::agent::capabilities::cancel_hook(&bus, ty, phase, ctx).await?;
        }
        // Bootstrap may assign a fresh id before any persist; wiped HOME may leave
        // an orphan id. Ensure an empty session so double-Esc opens an empty tree.
        self.agent
            .ensure_session(sid, None)
            .await
            .map_err(XyDriverError::from)?;
        let tree = match kind {
            SessionTreeKind::MessageHistory => self.store.message_history_tree(sid).await?,
            SessionTreeKind::FileBrowser => {
                return Err(XyDriverError::unsupported(session_tree_kind_unimplemented(
                    kind,
                )));
            }
        };
        if let Some(bus) = self.agent.hook_bus() {
            let kind = format!("{kind:?}");
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_tree(&kind);
            crate::agent::capabilities::observe_hook(&bus, ty, phase, ctx).await;
        }
        Ok(tree)
    }

    async fn travel_session_tree(
        &self,
        kind: SessionTreeKind,
        entry_id: &str,
    ) -> Result<SessionTreeTravel, XyDriverError> {
        let sid = self
            .agent
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        if let Some(bus) = self.agent.hook_bus() {
            let kind_s = format!("{kind:?}");
            let (ty, phase, ctx) =
                crate::agent::runtime::script_hook_ctx::session_before_tree_travel(
                    &kind_s, entry_id,
                );
            crate::agent::capabilities::cancel_hook(&bus, ty, phase, ctx).await?;
        }
        let travel = match kind {
            SessionTreeKind::MessageHistory => {
                let entries = self.store.load_entries(sid).await?;
                let travel = plan_message_history_travel(&entries, entry_id)?;
                self.store.set_leaf(sid, travel.leaf_id.as_deref());
                travel
            }
            SessionTreeKind::FileBrowser => {
                return Err(XyDriverError::unsupported(session_tree_kind_unimplemented(
                    kind,
                )));
            }
        };
        if let Some(bus) = self.agent.hook_bus() {
            let kind_s = format!("{kind:?}");
            let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::session_tree_travel(
                &kind_s,
                entry_id,
                travel.leaf_id.as_deref(),
            );
            crate::agent::capabilities::observe_hook(&bus, ty, phase, ctx).await;
        }
        Ok(travel)
    }

    async fn append_entry_label(
        &mut self,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XyDriverError> {
        use crate::protocol::session::{EntryBase, LabelEntry};

        let sid = self
            .agent
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        self.agent
            .ensure_session(sid, None)
            .await
            .map_err(XyDriverError::from)?;
        let entries = self.store.load_entries(sid).await?;
        if !entries.iter().any(|e| e.entry_id() == Some(target_id)) {
            return Err(XyDriverError::not_found(format!(
                "target entry not found: {target_id}"
            )));
        }
        let cleaned = label
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let entry = SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: "label".into(),
                id: String::new(),
                parent_id: None,
                timestamp: String::new(),
            },
            target_id: target_id.to_string(),
            label: cleaned,
        });
        Self::map_str(self.store.append_session_entry(sid, &entry).await)
    }

    fn leaf_entry_id(&self) -> Option<String> {
        let sid = self.agent.session_id()?;
        self.store.leaf_id(sid)
    }

    async fn load_debug_scene(&mut self, scene: &str) -> Result<DebugSceneLoad, XyDriverError> {
        use crate::app::debug_fixtures::{list_note, resolve_scene_id, seed_scene};

        let scene = scene.trim();
        if scene.is_empty() || scene.eq_ignore_ascii_case("list") {
            return Err(XyDriverError::invalid_input(list_note()));
        }
        let canonical = resolve_scene_id(scene).ok_or_else(|| {
            XyDriverError::invalid_input(format!("unknown debug scene: {scene}\n{}", list_note()))
        })?;
        let short = &uuid::Uuid::new_v4().to_string()[..8];
        let session_id = format!("debug-{canonical}-{short}");
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        self.store
            .create(&session_id, cwd.as_deref(), None)
            .await
            .map_err(XyDriverError::from)?;
        let canonical = seed_scene(self.store.as_ref(), &session_id, scene)
            .await
            .map_err(XyDriverError::from)?;
        bind_session_or_err(&mut self.agent, session_id.clone())?;
        let entries = self
            .store
            .load_entries(&session_id)
            .await
            .map_err(XyDriverError::from)?;
        let mut note = format!("debug scene `{canonical}` → session {session_id}");
        let model = match self.select_model("fake") {
            Ok(m) => {
                note.push_str("; model → fake");
                Some(m)
            }
            Err(_) => {
                note.push_str("; fake not in catalog (tree fixture only)");
                None
            }
        };
        Ok(DebugSceneLoad {
            session_id,
            entries,
            note,
            model,
        })
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XyDriverError> {
        Self::map_str(self.store.list_sessions().await)
    }

    async fn load_session_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XyDriverError> {
        Self::map_str(self.store.load_entries(session_id).await)
    }

    async fn new_session(&mut self) -> Result<String, XyDriverError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        self.store
            .create(&session_id, cwd.as_deref(), None)
            .await
            .map_err(XyDriverError::from)?;
        bind_session_or_err(&mut self.agent, session_id.clone())?;
        Ok(session_id)
    }

    async fn get_session_name(&self) -> Result<Option<String>, XyDriverError> {
        let sid = self
            .agent
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        Self::map_str(self.store.get_session_name(sid).await)
    }

    async fn set_session_name(&mut self, name: &str) -> Result<String, XyDriverError> {
        let sid = self
            .agent
            .session_id()
            .ok_or_else(|| XyDriverError::not_found("no active session"))?;
        let out = Self::map_str(self.store.set_session_name(sid, name).await)?;
        xylitol_ai_bridge::provider::set_obs_session_name(Some(out.as_str()));
        Ok(out)
    }

    async fn set_session_name_for(
        &mut self,
        session_id: &str,
        name: &str,
    ) -> Result<String, XyDriverError> {
        let out = Self::map_str(self.store.set_session_name(session_id, name).await)?;
        if self.agent.session_id() == Some(session_id) {
            xylitol_ai_bridge::provider::set_obs_session_name(Some(out.as_str()));
        }
        Ok(out)
    }

    async fn delete_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        Self::map_str(self.store.delete_session(session_id).await)
    }

    fn session_store(&self) -> Option<Arc<dyn XySessionStore>> {
        Some(self.store.clone())
    }

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        self.skill_catalog_pairs()
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        let skill_names = self.loaded_skill_names();
        let mcp_connecting_label = match &self.mcp_boot {
            McpBootState::Running { progress, .. } => progress.lock().await.connecting_label(),
            _ => None,
        };
        let connecting = matches!(self.mcp_boot, McpBootState::Running { .. });
        let mcp_bootstrap_complete =
            matches!(self.mcp_boot, McpBootState::Settled | McpBootState::Idle)
                && self.late_mcp_discover.is_none();
        let tools_table_frozen = self.agent.is_tools_frozen();
        let tool_names = self.agent.tool_names();
        let Some(state) = self.reload.as_ref() else {
            return LoadedResourcesSnapshot {
                skill_names,
                mcp_connecting_label,
                mcp_bootstrap_complete: true,
                tools_table_frozen,
                ..LoadedResourcesSnapshot::default()
            };
        };
        let connected = {
            let t0 = std::time::Instant::now();
            let c = state.mcp.connected_servers().await;
            crate::app::core::lag::note_detail(
                "loaded_snap_connected_servers",
                t0,
                &format!("servers={}", c.len()),
            );
            c
        };
        let diags = state.mcp.diagnostics().await;
        let mcp_servers = state
            .mcp_servers
            .iter()
            .map(|spec| {
                let id = spec.name.clone();
                let connected_info = connected.iter().find(|s| s.id == id);
                let failed = diags.iter().any(|d| d.server == id);
                let prefix = crate::protocol::mcp_tool_armed_prefix(&id);
                let armed_count = tool_names.iter().filter(|n| n.starts_with(&prefix)).count();
                let tools_armed = armed_count > 0;
                let phase = if connected_info.is_some() {
                    crate::app::core::driver::McpServerPhase::Connected
                } else if failed {
                    crate::app::core::driver::McpServerPhase::Failed
                } else if connecting {
                    crate::app::core::driver::McpServerPhase::Connecting
                } else {
                    crate::app::core::driver::McpServerPhase::Failed
                };
                crate::app::core::driver::McpServerSnapshot {
                    id,
                    phase,
                    tools_armed,
                    tool_count: connected_info.map(|c| c.tool_count).unwrap_or(armed_count),
                }
            })
            .collect();
        LoadedResourcesSnapshot {
            skill_names,
            mcp_connected: connected
                .into_iter()
                .map(|s| (s.id, s.tool_count))
                .collect(),
            mcp_configured: state.mcp_servers.len(),
            mcp_diag_short: diags
                .into_iter()
                .map(|d| format!("{}: {}", d.server, d.message))
                .collect(),
            mcp_connecting_label,
            mcp_servers,
            mcp_bootstrap_complete,
            tools_table_frozen,
        }
    }

    fn mcp_blocks_agent(&self) -> bool {
        matches!(
            self.mcp_boot,
            McpBootState::Running { .. }
                | McpBootState::Rebuilding { .. }
                | McpBootState::Settling { .. }
        )
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
        if !matches!(self.mcp_boot, McpBootState::Idle) {
            return;
        }
        let Some(state) = self.reload.as_ref() else {
            self.mcp_boot = McpBootState::Settled;
            return;
        };
        if state.mcp_servers.is_empty() {
            self.mcp_boot = McpBootState::Settled;
            return;
        }
        let servers = crate::app::core::mcp_spec::McpServerSpec::to_infra_list(&state.mcp_servers);
        let progress = Arc::new(tokio::sync::Mutex::new(
            crate::infra::mcp::McpConnectProgress {
                connecting: true,
                total: servers.len(),
                finished: 0,
                current: None,
            },
        ));
        let progress_task = progress.clone();
        let handle = tokio::spawn(async move {
            crate::infra::mcp::connect_and_discover_with_progress(&servers, Some(progress_task))
                .await
        });
        self.mcp_boot = McpBootState::Running {
            handle,
            progress,
            last_ui_label: None,
        };
    }

    async fn poll_mcp_bootstrap(&mut self) -> bool {
        // Late discover after gate timeout detached Running.
        if let Some(handle) = self.late_mcp_discover.as_ref()
            && handle.is_finished()
        {
            let handle = self.late_mcp_discover.take().expect("late handle");
            let mut refreshed = false;
            match handle.await {
                Ok(Ok(Some((manager, tools)))) => {
                    let tool_n = tools.len();
                    let old = self.reload.as_mut().and_then(|s| s.mcp.take_manager());
                    if let Some(old) = old {
                        tokio::spawn(async move {
                            old.shutdown().await;
                        });
                    }
                    if let Some(state) = self.reload.as_mut() {
                        state.mcp.set_manager(manager);
                    }
                    if self.agent.is_tools_frozen() {
                        log::info!(
                            target: "xylitol::mcp",
                            "late MCP discover after gate: registry only (FROZEN) mcp_tools={tool_n}"
                        );
                        refreshed = true;
                    } else {
                        let builtins = self.builtins_for_reload();
                        let set = tokio::task::spawn_blocking(move || {
                            crate::agent::tools::ToolSet::rebuild_agent_tools(builtins, tools)
                        })
                        .await;
                        if let Ok(set) = set {
                            let opts = self.agent.set_tools_defer_prompt(set);
                            let prompt = tokio::task::spawn_blocking(move || {
                                crate::agent::prompt::build_system_prompt(&opts)
                            })
                            .await;
                            if let Ok(prompt) = prompt {
                                self.agent.install_system_prompt_text(prompt);
                            }
                            refreshed = true;
                        }
                    }
                }
                Ok(Ok(None)) | Ok(Err(_)) | Err(_) => {
                    log::warn!(target: "xylitol::mcp", "late MCP discover after gate failed or empty");
                    refreshed = true;
                }
            }
            if refreshed {
                return true;
            }
        }

        // Finish deferred system-prompt install before polling connect progress.
        if matches!(self.mcp_boot, McpBootState::Settling { .. }) {
            let finished = match &self.mcp_boot {
                McpBootState::Settling { handle } => handle.is_finished(),
                _ => false,
            };
            if !finished {
                return false;
            }
            let prev = std::mem::replace(&mut self.mcp_boot, McpBootState::Idle);
            let McpBootState::Settling { handle } = prev else {
                return false;
            };
            match handle.await {
                Ok(prompt) => {
                    let t0 = std::time::Instant::now();
                    self.agent.install_system_prompt_text(prompt);
                    crate::app::core::lag::note("mcp_settle_install_prompt", t0);
                }
                Err(e) => {
                    log::warn!(target: "xylitol::mcp", "MCP settle prompt join failed: {e}");
                }
            }
            self.mcp_boot = McpBootState::Settled;
            // MUST refresh UI: bootstrap_complete flips Settling→Settled. Skipping
            // left a stale snap with mcp_bootstrap_complete=false so idle cue
            // sticky-restored "mcp pending" while the welcome card already showed
            // connected (manager/tools applied one phase earlier).
            let _ = self.try_complete_armed_tool_gate();
            return true;
        }

        // Apply rebuilt ToolSet + kick deferred prompt (rebuild ran off-tick).
        if matches!(self.mcp_boot, McpBootState::Rebuilding { .. }) {
            let finished = match &self.mcp_boot {
                McpBootState::Rebuilding { handle, .. } => handle.is_finished(),
                _ => false,
            };
            if !finished {
                return false;
            }
            let prev = std::mem::replace(&mut self.mcp_boot, McpBootState::Idle);
            let McpBootState::Rebuilding {
                handle,
                manager,
                tool_n,
                settle_started,
            } = prev
            else {
                return false;
            };
            let set = match handle.await {
                Ok(set) => set,
                Err(e) => {
                    log::warn!(target: "xylitol::mcp", "MCP settle rebuild join failed: {e}");
                    self.mcp_boot = McpBootState::Settled;
                    return true;
                }
            };
            crate::app::core::lag::note_detail(
                "mcp_settle_rebuild_tools",
                settle_started,
                &format!("mcp_tools={tool_n}"),
            );
            if let Some(state) = self.reload.as_mut() {
                state.mcp.set_manager(manager);
            }
            // c1900: after FROZEN, settle updates registry only — no provider tools expand.
            if self.agent.is_tools_frozen() {
                log::info!(
                    target: "xylitol::mcp",
                    "MCP settle ignored for provider tools (FROZEN); registry updated mcp_tools={tool_n}"
                );
                self.mcp_boot = McpBootState::Settled;
                return true;
            }
            let t_set = std::time::Instant::now();
            let opts = self.agent.set_tools_defer_prompt(set);
            crate::app::core::lag::note_detail(
                "mcp_settle_set_tools",
                t_set,
                &format!("mcp_tools={tool_n}"),
            );
            let handle = tokio::task::spawn_blocking(move || {
                crate::agent::prompt::build_system_prompt(&opts)
            });
            self.mcp_boot = McpBootState::Settling { handle };
            crate::app::core::lag::note_detail(
                "mcp_settle_total",
                settle_started,
                &format!("mcp_tools={tool_n} deferred_rebuild=1 deferred_prompt=1"),
            );
            return true;
        }

        let finished = match &self.mcp_boot {
            McpBootState::Running { handle, .. } => handle.is_finished(),
            _ => return false,
        };
        if !finished {
            // Progress-only: refresh loaded-resources when the connecting label
            // changes (0/n → 1/n …). Unchanged ticks MUST NOT invalidate TUI upper.
            let label = match &self.mcp_boot {
                McpBootState::Running { progress, .. } => progress.lock().await.connecting_label(),
                _ => return false,
            };
            let progress_refresh =
                if let McpBootState::Running { last_ui_label, .. } = &mut self.mcp_boot {
                    mcp_progress_needs_ui_refresh(last_ui_label, label)
                } else {
                    false
                };
            let gated = self.try_complete_armed_tool_gate();
            return progress_refresh || gated;
        }
        let prev = std::mem::replace(&mut self.mcp_boot, McpBootState::Idle);
        let McpBootState::Running { handle, .. } = prev else {
            return false;
        };
        let boot_refreshed = match handle.await {
            Ok(Ok(Some((manager, tools)))) => {
                let tool_n = tools.len();
                let settle_started = std::time::Instant::now();
                let old = self.reload.as_mut().and_then(|s| s.mcp.take_manager());
                // Do not await shutdown on the TUI tick path (spinner hitch).
                if let Some(old) = old {
                    tokio::spawn(async move {
                        old.shutdown().await;
                    });
                }
                let builtins = self.builtins_for_reload();
                let handle = tokio::task::spawn_blocking(move || {
                    crate::agent::tools::ToolSet::rebuild_agent_tools(builtins, tools)
                });
                self.mcp_boot = McpBootState::Rebuilding {
                    handle,
                    manager,
                    tool_n,
                    settle_started,
                };
                // UI refresh waits until tools are applied (Rebuilding → Settling).
                false
            }
            Ok(Ok(None)) => {
                self.mcp_boot = McpBootState::Settled;
                true
            }
            Ok(Err(e)) => {
                log::warn!(target: "xylitol::mcp", "MCP bootstrap failed: {e}");
                self.mcp_boot = McpBootState::Settled;
                true
            }
            Err(e) => {
                log::warn!(target: "xylitol::mcp", "MCP bootstrap join failed: {e}");
                self.mcp_boot = McpBootState::Settled;
                true
            }
        };
        self.try_complete_armed_tool_gate() || boot_refreshed
    }

    async fn reload_runtime(&mut self) -> Result<RuntimeReloadReport, XyDriverError> {
        let Some(mut state) = self.reload.take() else {
            return Ok(RuntimeReloadReport::noop());
        };

        let mut steps = Vec::new();

        // Re-read trust store so `/trust` + later `/reload` picks up new decisions (c1105).
        let trust_mgr = crate::infra::trust::TrustManager::new(
            crate::infra::trust::TrustManager::default_dir(),
        );
        let cwd_str = state.cwd.display().to_string();
        state.project_trusted = trust_mgr.is_trusted(&cwd_str);

        let app_config = crate::infra::config::loader::load_app_config(None).ok();
        state.mcp_servers = crate::app::core::mcp_spec::McpServerSpec::from_infra_list(
            app_config.as_ref().and_then(|c| c.mcp_servers.clone()),
        )
        .unwrap_or_default();
        let config_system_prompt = app_config
            .as_ref()
            .and_then(|cfg| cfg.resolve_default_profile().ok())
            .and_then(|p| p.system_prompt.clone());

        let skills = crate::app::core::bootstrap::reload_skills(
            self,
            &state.cwd,
            &state.agent_dir,
            state.project_trusted,
        );
        let skills_msg = if skills.names.is_empty() {
            "0 skills".into()
        } else {
            format!("{} skill(s): {}", skills.count, skills.names.join(", "))
        };
        steps.push(ReloadStepReport {
            step: "skills",
            ok: true,
            message: skills_msg,
        });

        match state.mcp.reload(self, &state.mcp_servers).await {
            Ok(()) => {
                // c1900: reload is an explicit re-freeze epoch; bootstrap mark settled.
                self.mcp_boot = McpBootState::Settled;
                let connected = state.mcp.connected_servers().await;
                let diags = state.mcp.diagnostics().await;
                if diags.is_empty() {
                    let ids: Vec<_> = connected.iter().map(|s| s.id.as_str()).collect();
                    steps.push(ReloadStepReport {
                        step: "mcp",
                        ok: true,
                        message: format!(
                            "{} configured, {} connected [{}]",
                            state.mcp_servers.len(),
                            connected.len(),
                            ids.join(", ")
                        ),
                    });
                } else {
                    let detail = diags
                        .iter()
                        .map(|d| format!("{}: {}", d.server, d.message))
                        .collect::<Vec<_>>()
                        .join("; ");
                    steps.push(ReloadStepReport {
                        step: "mcp",
                        ok: !connected.is_empty(),
                        message: format!(
                            "{} configured, {} connected; diagnostics: {detail}",
                            state.mcp_servers.len(),
                            connected.len()
                        ),
                    });
                }
            }
            Err(e) => steps.push(ReloadStepReport {
                step: "mcp",
                ok: false,
                message: e.to_string(),
            }),
        }

        let ctx = crate::app::core::bootstrap::reload_prompt_context(
            self,
            &state.cwd,
            &state.agent_dir,
            state.project_trusted,
            config_system_prompt,
        );
        steps.push(ReloadStepReport {
            step: "context",
            ok: true,
            message: format!(
                "{} context file(s), system={}, append={}",
                ctx.context_file_count, ctx.has_system_prompt, ctx.append_count
            ),
        });

        self.reload = Some(state);
        Ok(RuntimeReloadReport { steps })
    }

    fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        let cwd = self
            .reload
            .as_ref()
            .map(|s| s.cwd.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let cwd_str = cwd.display().to_string();
        let mgr = crate::infra::trust::TrustManager::new(
            crate::infra::trust::TrustManager::default_dir(),
        );
        let options = mgr.get_trust_options(&cwd_str, false);
        let opt = match mode {
            ProjectTrustMode::TrustCwd => options.first(),
            ProjectTrustMode::TrustParent => {
                options.iter().find(|o| o.label.starts_with("Trust parent"))
            }
            ProjectTrustMode::Deny => options.iter().find(|o| o.label == "Do not trust"),
        };
        let Some(opt) = opt else {
            return Err(match mode {
                ProjectTrustMode::TrustParent => {
                    XyDriverError::invalid_input("no parent folder to trust")
                }
                _ => XyDriverError::invalid_input("trust option unavailable"),
            });
        };
        if !opt.updates.is_empty() {
            mgr.apply_updates(&opt.updates)
                .map_err(|e| XyDriverError::io(format!("trust store write failed: {e}")))?;
        }
        let saved = opt.saved_path.clone().unwrap_or_else(|| cwd_str.clone());
        let verb = if opt.trusted { "trusted" } else { "denied" };
        Ok(ProjectTrustPersistReport {
            trusted: opt.trusted,
            saved_path: opt.saved_path.clone(),
            message: format!(
                "Project {verb} at {saved}. {}",
                ProjectTrustPersistReport::RELOAD_HINT
            ),
        })
    }

    async fn copy_text_to_clipboard(
        &mut self,
        text: &str,
    ) -> Result<ClipboardCopyOutcome, XyDriverError> {
        let plan = crate::infra::clipboard::plan_clipboard_copy_async(text.to_string())
            .await
            .map_err(XyDriverError::io)?;
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
            .map_err(XyDriverError::io)?;
        let Some(image) = image else {
            return Ok(None);
        };
        let path = tokio::task::spawn_blocking(move || {
            crate::infra::clipboard::write_clipboard_image_temp(&image.bytes, &image.mime_type)
        })
        .await
        .map_err(|e| XyDriverError::io(format!("clipboard image write task failed: {e}")))?
        .map_err(XyDriverError::io)?;
        Ok(Some(path))
    }

    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        tokio::task::spawn_blocking(crate::infra::clipboard::read_clipboard_text)
            .await
            .map_err(|e| XyDriverError::io(format!("clipboard text task failed: {e}")))?
            .map_err(XyDriverError::io)
    }
}

#[cfg(test)]
mod driver_session_tree_tests {
    use serial_test::serial;
    use std::sync::Arc;

    use super::*;
    use crate::agent::AgentBuilder;
    use crate::agent::tools::ToolSet;
    use crate::infra::config::value::InfraSecretResolver;
    use crate::infra::event::EventBus;
    use crate::infra::permission;
    use crate::infra::session::SessionManager;
    use crate::protocol::model::XyModelConfig;
    use crate::protocol::ports::{XyEventSink, XyModel, XySessionStore};
    use crate::protocol::session::{
        EntryBase, MessageEntry, SessionEntry, SessionTreeKind, ThinkingLevelChangeEntry,
    };

    type ModelBuilderFn =
        Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>;

    fn msg_entry(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: format!("2026-01-01T00:00:00.{id}Z"),
            },
            message: crate::protocol::session::fixture_message_json(role, text),
        })
    }

    async fn build_test_driver(store: Arc<SessionManager>) -> XyInProcessDriver {
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut agent = AgentBuilder::new(
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new())),
            Arc::new(crate::infra::provider::factory::build_provider),
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
        .build()
        .expect("build agent");
        let sid = uuid::Uuid::new_v4().to_string();
        store_trait
            .create(&sid, Some("."), None)
            .await
            .expect("create session");
        agent.bind_session(sid).expect("bind_session");
        XyInProcessDriver::new(agent, store)
    }

    #[tokio::test]
    #[serial]
    async fn switch_session_restores_sticky_thinking_without_rewriting() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store.clone()).await;
        let target = "restored-thinking";
        store.create(target, Some("."), None).await.unwrap();
        store
            .append(
                target,
                &SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
                    base: EntryBase {
                        entry_type: "thinking_level_change".into(),
                        id: String::new(),
                        parent_id: None,
                        timestamp: String::new(),
                    },
                    thinking_level: "vendor-retired".into(),
                }),
            )
            .await
            .unwrap();
        store
            .append(target, &msg_entry("a1", None, "assistant", "flush"))
            .await
            .unwrap();
        let session_file = store.get_session_file(target).unwrap();
        let before = std::fs::read(&session_file).unwrap();

        driver.switch_session(target).await.unwrap();

        assert_eq!(driver.thinking_level(), "vendor-retired");
        assert_eq!(std::fs::read(session_file).unwrap(), before);
    }

    #[tokio::test]
    #[serial]
    async fn in_process_session_tree_ensures_missing_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut agent = AgentBuilder::new(
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new())),
            Arc::new(crate::infra::provider::factory::build_provider),
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
        .build()
        .expect("build agent");
        // Orphan id: set on agent but never created on disk (wipe / pre-persist).
        let orphan = uuid::Uuid::new_v4().to_string();
        agent.bind_session(orphan.clone()).expect("bind_session");
        let driver = XyInProcessDriver::new(agent, store);
        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("empty tree after ensure");
        assert!(tree.is_empty(), "expected empty tree, got {tree:?}");
        assert!(
            store_trait.exists(&orphan).await,
            "ensure_session must create orphan session"
        );
    }

    #[tokio::test]
    #[serial]
    async fn in_process_session_tree_returns_parent_child() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "hello"))
            .await
            .expect("append u1");
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "hi"))
            .await
            .expect("append a1");

        let loaded = store.load(&sid).await.expect("load after append");
        let ids: Vec<_> = loaded.iter().filter_map(|e| e.entry_id()).collect();
        assert_eq!(
            ids,
            vec!["u1", "a1"],
            "persisted entries before tree: {loaded:?}"
        );

        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("tree");
        assert_eq!(tree.len(), 1, "tree={tree:?} loaded={loaded:?}");
        assert_eq!(tree[0].entry.entry_id(), Some("u1"));
        assert_eq!(
            tree[0].children.len(),
            1,
            "expected a1 under u1; tree={tree:?} loaded={loaded:?}"
        );
        assert_eq!(tree[0].children[0].entry.entry_id(), Some("a1"));
    }

    #[tokio::test]
    #[serial]
    async fn in_process_travel_user_sets_parent_leaf_and_editor_text() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "edit me"))
            .await
            .unwrap();
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "reply"))
            .await
            .unwrap();
        XySessionStore::set_leaf(store.as_ref(), &sid, Some("a1"));

        let travel = driver
            .travel_session_tree(SessionTreeKind::MessageHistory, "u1")
            .await
            .expect("travel");
        assert_eq!(travel.leaf_id, None);
        assert_eq!(travel.editor_text.as_deref(), Some("edit me"));
        assert_eq!(XySessionStore::leaf_id(store.as_ref(), &sid), None);
    }

    #[tokio::test]
    #[serial]
    async fn in_process_travel_non_user_sets_leaf_without_editor_text() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");

        store
            .append_with_id(&sid, &msg_entry("u1", None, "user", "hello"))
            .await
            .unwrap();
        store
            .append_with_id(&sid, &msg_entry("a1", Some("u1"), "assistant", "reply"))
            .await
            .unwrap();

        let travel = driver
            .travel_session_tree(SessionTreeKind::MessageHistory, "a1")
            .await
            .expect("travel");
        assert_eq!(travel.leaf_id.as_deref(), Some("a1"));
        assert!(travel.editor_text.is_none());
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("a1")
        );
    }

    #[tokio::test]
    #[serial]
    async fn unsupported_tree_kind_returns_err_without_changing_leaf() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");
        XySessionStore::set_leaf(store.as_ref(), &sid, Some("keep"));

        let err = driver
            .session_tree(SessionTreeKind::FileBrowser)
            .await
            .expect_err("file_browser");
        assert!(err.to_string().contains("file_browser"));
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("keep")
        );

        let err = driver
            .travel_session_tree(SessionTreeKind::FileBrowser, "x")
            .await
            .expect_err("travel file_browser");
        assert!(err.to_string().contains("file_browser"));
        assert_eq!(
            XySessionStore::leaf_id(store.as_ref(), &sid).as_deref(),
            Some("keep")
        );
    }

    #[tokio::test]
    #[serial]
    async fn after_run_session_tree_reflects_persisted_turn() {
        use std::pin::Pin;

        use async_trait::async_trait;
        use futures::StreamExt;

        use crate::protocol::error::XyError;
        use crate::protocol::message::XyStopReason;
        use crate::protocol::model::XyModelConfig;
        use crate::protocol::model::{XyChunk, XyModelMeta, XyToolSchema};
        use crate::protocol::ports::{XyModel, XyStream};

        struct TextMockModel;
        #[async_trait]
        impl XyModel for TextMockModel {
            fn name(&self) -> &str {
                "text-mock"
            }

            async fn generate_stream(
                &self,
                _messages: Vec<crate::protocol::message::LlmMessage>,
                _tools: &[XyToolSchema],
                _stream: bool,
                _options: crate::protocol::ports::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                let chunks = vec![
                    Ok(XyChunk::TextDelta("reply".into())),
                    Ok(XyChunk::Done {
                        finish_reason: XyStopReason::Stop,
                        usage: None,
                    }),
                ];
                Ok(Box::pin(futures::stream::iter(chunks))
                    as Pin<
                        Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>,
                    >)
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut reg =
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: XyModelConfig {
                kind: crate::protocol::model::XyModelKind::Fake,
                api_key: String::new(),
                model: "mock".into(),
                base_url: None,
                api: None,
                compat: None,
            },
            display_name: "Mock".into(),
            thinking: false,
            context_window: 128000,
            api: String::new(),
            provider: String::new(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
            thinking_level_map: Default::default(),
        });
        let builder: ModelBuilderFn = Arc::new(|_| Ok(Arc::new(TextMockModel) as Arc<dyn XyModel>));
        let mut agent = AgentBuilder::new(
            reg,
            builder,
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn crate::protocol::ports::XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::empty())
        .build()
        .expect("build agent");
        agent.select_model("mock").expect("select mock");
        let sid = uuid::Uuid::new_v4().to_string();
        agent.bind_session(sid).expect("bind_session");
        let mut driver = XyInProcessDriver::new(agent, store_trait);

        let mut stream = driver.run("hello tree").await;
        while stream.next().await.is_some() {}

        let tree = driver
            .session_tree(SessionTreeKind::MessageHistory)
            .await
            .expect("tree");
        assert!(
            !tree.is_empty(),
            "session tree should reflect persisted messages"
        );
    }

    #[tokio::test]
    #[serial]
    async fn fork_rejects_unflushed_session_via_driver() {
        // TUI cannot hit this while assistant is streaming (steer takes over); cover via XyDriver.
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store.clone()).await;
        let sid = driver.session_id().expect("session");
        let path = store.get_session_file(&sid).expect("persisted path");
        assert!(
            !path.exists(),
            "build_test_driver leaves session pending-only"
        );

        store
            .append(
                &sid,
                &SessionEntry::Message(MessageEntry {
                    base: EntryBase {
                        entry_type: "message".into(),
                        id: String::new(),
                        parent_id: None,
                        timestamp: String::new(),
                    },
                    message: crate::protocol::session::fixture_message_json("user", "only user"),
                }),
            )
            .await
            .expect("pending user");
        assert!(!path.exists());

        let uid = store
            .load(&sid)
            .await
            .expect("load")
            .iter()
            .find_map(|e| e.entry_id())
            .expect("user id")
            .to_string();

        let err = driver
            .fork_session(&uid, crate::protocol::session::ForkPosition::At)
            .await
            .expect_err("unflushed fork");
        assert!(
            err.to_string().contains("not been saved yet"),
            "pi unflushed guard via XyDriver: {err}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn persist_project_trust_writes_store_under_home() {
        let home = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", home.path());
        }
        let cwd = home.path().join("proj");
        std::fs::create_dir_all(cwd.join(".xylitol")).unwrap();
        let store = Arc::new(SessionManager::new(home.path().join("sessions")));
        let mut driver = build_test_driver(store).await;
        driver.enable_reload_state(cwd.clone(), home.path().join(".xylitol"), false, Vec::new());
        let report = driver
            .persist_project_trust(ProjectTrustMode::TrustCwd)
            .expect("persist");
        assert!(report.trusted);
        assert!(report.message.contains("/reload") || report.message.contains("restart"));
        let mgr = crate::infra::trust::TrustManager::new(
            crate::infra::trust::TrustManager::default_dir(),
        );
        assert!(mgr.is_trusted(&cwd.display().to_string()));
        match prev {
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    #[tokio::test]
    #[serial]
    async fn begin_mcp_bootstrap_empty_settles_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store).await;
        driver.enable_reload_state(
            dir.path().to_path_buf(),
            dir.path().join(".xylitol"),
            true,
            Vec::new(),
        );
        assert!(!driver.mcp_blocks_agent());
        driver.begin_mcp_bootstrap().await;
        assert!(
            !driver.mcp_blocks_agent(),
            "empty mcp_servers MUST settle without gating"
        );
        let snap = driver.loaded_resources_snapshot().await;
        assert!(snap.mcp_connecting_label.is_none());
        assert_eq!(snap.mcp_configured, 0);
    }

    #[tokio::test]
    #[serial]
    async fn mcp_settle_defers_system_prompt_off_tick() {
        use crate::app::core::mcp_spec::{McpServerSpec, McpTransportSpec};

        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store).await;
        let bad = McpServerSpec {
            name: "bad".into(),
            transport: McpTransportSpec::Stdio,
            command: None,
            args: None,
            url: None,
            env: None,
            headers: None,
        };
        driver.enable_reload_state(
            dir.path().to_path_buf(),
            dir.path().join(".xylitol"),
            true,
            vec![bad],
        );
        let prompt_before = driver.system_prompt_for_test();
        assert!(prompt_before.is_some());

        driver.begin_mcp_bootstrap().await;
        assert!(driver.mcp_blocks_agent(), "Running must gate wait_mcp");

        // Durable sticky-cue contract: any poll that leaves mcp_blocks_agent
        // (Settling→Settled, or failed Running→Settled) MUST return true so the
        // TUI host refreshes loaded-resources (mcp_bootstrap_complete flips).
        let mut saw_ungate_with_refresh = false;
        let mut saw_tools_while_gated = false;
        for _ in 0..500 {
            let before = driver.mcp_blocks_agent();
            let refresh = driver.poll_mcp_bootstrap().await;
            let after = driver.mcp_blocks_agent();
            if before
                && driver.tool_names_for_test().iter().any(|n| n == "read")
                && !driver
                    .loaded_resources_snapshot()
                    .await
                    .mcp_bootstrap_complete
            {
                saw_tools_while_gated = true;
            }
            if before && !after {
                assert!(
                    refresh,
                    "leaving mcp_blocks_agent MUST return true (Settling→Settled sticky cue)"
                );
                saw_ungate_with_refresh = true;
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            saw_tools_while_gated,
            "must observe tools applied while still gated (Settling)"
        );
        assert!(
            saw_ungate_with_refresh,
            "must observe gated→ungated poll with refresh=true"
        );
        assert!(!driver.mcp_blocks_agent());
        let snap = driver.loaded_resources_snapshot().await;
        assert!(
            snap.mcp_bootstrap_complete,
            "after ungate, bootstrap MUST be complete"
        );
        assert!(
            !snap.mcp_tools_pending(),
            "settled+complete MUST clear mcp_tools_pending"
        );
        assert!(driver.system_prompt_for_test().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn leaving_mcp_gate_must_signal_ui_refresh() {
        use crate::app::core::mcp_spec::{McpServerSpec, McpTransportSpec};

        // Narrow regression for sticky "mcp pending" after welcome shows connected:
        // host only refreshes when poll_mcp_bootstrap returns true.
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store).await;
        driver.enable_reload_state(
            dir.path().to_path_buf(),
            dir.path().join(".xylitol"),
            true,
            vec![McpServerSpec {
                name: "bad".into(),
                transport: McpTransportSpec::Stdio,
                command: None,
                args: None,
                url: None,
                env: None,
                headers: None,
            }],
        );
        driver.begin_mcp_bootstrap().await;
        assert!(driver.mcp_blocks_agent());

        let mut ok = false;
        for _ in 0..500 {
            let before = driver.mcp_blocks_agent();
            let refresh = driver.poll_mcp_bootstrap().await;
            let after = driver.mcp_blocks_agent();
            if before && !after {
                assert!(
                    refresh,
                    "Settling→Settled (or Running fail→Settled) MUST signal UI refresh"
                );
                ok = true;
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(ok, "gated→ungated transition not observed");
        let snap = driver.loaded_resources_snapshot().await;
        assert!(snap.mcp_bootstrap_complete);
        assert!(!snap.mcp_tools_pending());
    }

    #[test]
    #[serial]
    fn mcp_progress_needs_ui_refresh_only_on_label_change() {
        let mut last = None;
        assert!(mcp_progress_needs_ui_refresh(
            &mut last,
            Some("connecting 0/2".into())
        ));
        assert!(!mcp_progress_needs_ui_refresh(
            &mut last,
            Some("connecting 0/2".into())
        ));
        assert!(mcp_progress_needs_ui_refresh(
            &mut last,
            Some("connecting 1/2".into())
        ));
        assert!(!mcp_progress_needs_ui_refresh(
            &mut last,
            Some("connecting 1/2".into())
        ));
        assert!(mcp_progress_needs_ui_refresh(&mut last, None));
    }

    #[test]
    #[serial]
    fn mcp_tools_pending_ignores_failed_when_bootstrap_complete() {
        use crate::app::core::driver::{McpServerPhase, McpServerSnapshot};

        let snap = LoadedResourcesSnapshot {
            mcp_configured: 2,
            mcp_bootstrap_complete: true,
            tools_table_frozen: true,
            mcp_servers: vec![
                McpServerSnapshot {
                    id: "ok".into(),
                    phase: McpServerPhase::Connected,
                    tools_armed: true,
                    tool_count: 1,
                },
                McpServerSnapshot {
                    id: "bad".into(),
                    phase: McpServerPhase::Failed,
                    tools_armed: false,
                    tool_count: 0,
                },
            ],
            ..Default::default()
        };
        assert!(!snap.mcp_tools_pending());
    }

    #[test]
    #[serial]
    fn mcp_tools_pending_while_connecting_label() {
        let snap = LoadedResourcesSnapshot {
            mcp_configured: 2,
            mcp_connecting_label: Some("connecting 1/2".into()),
            mcp_bootstrap_complete: false,
            ..Default::default()
        };
        assert!(snap.mcp_tools_pending());
    }

    #[test]
    #[serial]
    fn mcp_tools_pending_pre_freeze_while_settling_even_if_armed() {
        use crate::app::core::driver::{McpServerPhase, McpServerSnapshot};

        // Resume: prior mcp tools still armed, bootstrap Settling (label cleared).
        let snap = LoadedResourcesSnapshot {
            mcp_configured: 1,
            mcp_bootstrap_complete: false,
            tools_table_frozen: false,
            mcp_servers: vec![McpServerSnapshot {
                id: "fs".into(),
                phase: McpServerPhase::Connected,
                tools_armed: true,
                tool_count: 3,
            }],
            ..Default::default()
        };
        assert!(
            snap.mcp_tools_pending(),
            "pre-freeze incomplete bootstrap MUST keep mcp pending (Assembling + cue)"
        );
    }

    #[test]
    #[serial]
    fn mcp_tools_pending_clears_after_freeze_when_complete() {
        use crate::app::core::driver::{McpServerPhase, McpServerSnapshot};

        let snap = LoadedResourcesSnapshot {
            mcp_configured: 1,
            mcp_bootstrap_complete: true,
            tools_table_frozen: true,
            mcp_servers: vec![McpServerSnapshot {
                id: "fs".into(),
                phase: McpServerPhase::Connected,
                tools_armed: true,
                tool_count: 3,
            }],
            ..Default::default()
        };
        assert!(!snap.mcp_tools_pending());
    }

    #[tokio::test]
    #[serial]
    async fn ensure_freeze_on_empty_mcp_and_ignore_expand() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store).await;
        driver.enable_reload_state(
            dir.path().to_path_buf(),
            dir.path().join(".xylitol"),
            true,
            Vec::new(),
        );
        assert!(!driver.is_tools_frozen());
        driver.begin_mcp_bootstrap().await;
        driver.ensure_tool_table_frozen().await;
        assert!(driver.is_tools_frozen());
        let names = driver.tool_names_for_test();
        assert!(names.iter().any(|n| n == "read"));
        let n = names.len();

        driver.set_tools(ToolSet::empty());
        assert_eq!(driver.tool_names_for_test().len(), n);
        assert!(driver.is_tools_frozen());
    }

    #[tokio::test]
    #[serial]
    async fn reload_re_freezes_tools() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store).await;
        driver.enable_reload_state(
            dir.path().to_path_buf(),
            dir.path().join(".xylitol"),
            true,
            Vec::new(),
        );
        driver.ensure_tool_table_frozen().await;
        assert!(driver.is_tools_frozen());
        let report = driver.reload_runtime().await.expect("reload");
        assert!(report.steps.iter().any(|s| s.step == "mcp"));
        assert!(driver.is_tools_frozen());
        assert!(driver.tool_names_for_test().iter().any(|n| n == "read"));
    }

    #[tokio::test]
    #[serial]
    async fn arm_tool_freeze_gate_empty_mcp_freezes_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let mut driver = build_test_driver(store).await;
        driver.enable_reload_state(
            dir.path().to_path_buf(),
            dir.path().join(".xylitol"),
            true,
            Vec::new(),
        );
        assert!(!driver.is_tools_frozen());
        driver.arm_tool_freeze_gate_inner().await;
        assert!(driver.is_tools_frozen());
        let snap = driver.loaded_resources_snapshot().await;
        assert!(snap.mcp_connecting_label.is_none());
        assert!(snap.mcp_bootstrap_complete);
    }

    /// Default `XyDriver::run` path is Reject: concurrent root while live → Busy, one provider stream.
    #[tokio::test]
    #[serial]
    async fn concurrent_run_rejects_second_with_busy() {
        use std::pin::Pin;
        use std::sync::atomic::{AtomicUsize, Ordering};

        use async_trait::async_trait;
        use futures::StreamExt;

        use crate::protocol::error::XyError;
        use crate::protocol::message::XyStopReason;
        use crate::protocol::model::{XyChunk, XyModelConfig, XyModelMeta, XyToolSchema};
        use crate::protocol::ports::{XyModel, XyStream};

        struct SlowMock {
            calls: Arc<AtomicUsize>,
        }
        #[async_trait]
        impl XyModel for SlowMock {
            fn name(&self) -> &str {
                "slow-mock"
            }
            async fn generate_stream(
                &self,
                _messages: Vec<crate::protocol::message::LlmMessage>,
                _tools: &[XyToolSchema],
                _stream: bool,
                _options: crate::protocol::ports::XyGenerateOptions,
            ) -> Result<XyStream, XyError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(Box::pin(async_stream::stream! {
                    for i in 0..40u32 {
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        yield Ok(XyChunk::TextDelta(format!("c{i}")));
                    }
                    yield Ok(XyChunk::Done {
                        finish_reason: XyStopReason::Stop,
                        usage: None,
                    });
                })
                    as Pin<
                        Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>,
                    >)
            }
        }

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_b = calls.clone();
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
        let store_trait: Arc<dyn XySessionStore> = store.clone();
        let mut reg =
            crate::agent::model::registry::ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: XyModelConfig {
                kind: crate::protocol::model::XyModelKind::Fake,
                api_key: String::new(),
                model: "mock".into(),
                base_url: None,
                api: None,
                compat: None,
            },
            display_name: "Mock".into(),
            thinking: false,
            context_window: 128000,
            api: String::new(),
            provider: String::new(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
            thinking_level_map: Default::default(),
        });
        let builder: ModelBuilderFn = Arc::new(move |_| {
            Ok(Arc::new(SlowMock {
                calls: calls_b.clone(),
            }) as Arc<dyn XyModel>)
        });
        let mut agent = AgentBuilder::new(
            reg,
            builder,
            store_trait.clone(),
            Arc::new(EventBus::new()) as Arc<dyn crate::protocol::ports::XyEventSink>,
            permission::allow_all_permission(),
        )
        .cwd(".")
        .tools(ToolSet::empty())
        .build()
        .expect("build agent");
        agent.select_model("mock").expect("select mock");
        let sid = uuid::Uuid::new_v4().to_string();
        agent.bind_session(sid).expect("bind_session");
        let mut driver = XyInProcessDriver::new(agent, store_trait);

        let mut first = driver.run("one").await;
        let mut saw = false;
        while let Some(ev) = first.next().await {
            if matches!(ev, crate::protocol::lifecycle::XyEvent::TextDelta(_)) {
                saw = true;
                break;
            }
        }
        assert!(saw, "first driver run must emit text");

        let mut second = driver.run("two").await;
        let mut busy = false;
        let mut second_text = false;
        while let Some(ev) = second.next().await {
            match ev {
                crate::protocol::lifecycle::XyEvent::Error(err) if err.kind == "Busy" => {
                    busy = true
                }
                crate::protocol::lifecycle::XyEvent::TextDelta(_) => second_text = true,
                _ => {}
            }
        }
        assert!(busy, "second XyDriver::run must Busy");
        assert!(!second_text, "rejected run must not stream model text");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "only one provider stream");

        while first.next().await.is_some() {}
    }
}
