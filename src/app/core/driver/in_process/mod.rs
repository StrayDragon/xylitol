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

use super::types::{
    ClipboardCopyOutcome, CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo,
    ProjectTrustMode, ProjectTrustPersistReport, RuntimeReloadReport,
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
    /// Interactive bang output-event sink (c2760): runtime-injected, `&self`
    /// bash execution pushes chunks into it while running.
    bash_run_sink: std::sync::Mutex<Option<crate::protocol::ports::BashOutputSink>>,
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
            bash_run_sink: std::sync::Mutex::new(None),
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

    #[cfg(test)]
    pub(crate) fn agent_cwd_for_test(&self) -> String {
        self.agent.cwd().to_string()
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

    /// Composition-root assembly: bind the configured default model without
    /// persisting a `modelChange` row (attach-time restore, not a user change).
    pub async fn restore_model(&mut self, model_id: &str) -> Result<(), XyDriverError> {
        self.agent
            .select_model_with_source(model_id, "restore")
            .await
            .map_err(XyDriverError::from)
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
            if let Err(e) = bind_session_or_err(&mut self.agent, id.clone()) {
                e.log_failure("driver.run.bind_session");
            }
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

    fn thinking_level(&self) -> String {
        self.agent.thinking_level()
    }

    fn session_id(&self) -> Option<String> {
        self.agent.session_id().map(String::from)
    }

    fn set_bash_run_sink(&mut self, sink: Option<crate::protocol::ports::BashOutputSink>) {
        *self.bash_run_sink.lock().unwrap_or_else(|e| e.into_inner()) = sink;
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

    fn leaf_entry_id(&self) -> Option<String> {
        XyInProcessDriver::leaf_entry_id(self)
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

    async fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        XyInProcessDriver::persist_project_trust(self, mode)
    }

    async fn copy_text_to_clipboard(
        &mut self,
        text: &str,
    ) -> Result<ClipboardCopyOutcome, XyDriverError> {
        super::clipboard::copy_text_to_clipboard(text.to_string()).await
    }

    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, XyDriverError> {
        super::clipboard::stage_clipboard_image().await
    }

    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        super::clipboard::read_clipboard_text().await
    }
}

// ── Session operations: inherent (Command executor SSOT, c2710) ─────
impl XyInProcessDriver {
    pub(crate) async fn select_model(
        &mut self,
        model_id: &str,
    ) -> Result<ModelInfo, XyDriverError> {
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

    pub(crate) async fn cycle_model(&mut self) -> Result<ModelInfo, XyDriverError> {
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

    pub(crate) async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        self.agent
            .set_thinking_level(level)
            .await
            .map_err(XyDriverError::from)
    }

    pub(crate) async fn compact(
        &mut self,
        instructions: Option<String>,
    ) -> Result<bool, XyDriverError> {
        // Force path (c1640 / pi compact) — MUST NOT use maybe_auto_compact.
        self.agent
            .force_compact(instructions)
            .await
            .map(|()| true)
            .map_err(XyDriverError::from)
    }

    pub(crate) async fn export_html(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = require_active_session(&self.agent)?.to_string();
        self.exporter
            .export_to_html(self.store.as_ref(), &sid, path)
            .await?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub(crate) async fn export_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        let sid = require_active_session(&self.agent)?.to_string();
        self.exporter
            .export_to_jsonl(self.store.as_ref(), &sid, path)
            .await?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub(crate) async fn import_jsonl(&mut self, path: &Path) -> Result<String, XyDriverError> {
        self.exporter
            .import_from_jsonl(self.store.as_ref(), path)
            .await
    }

    pub(crate) async fn steer(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.agent.steer(message);
        Ok(())
    }

    pub(crate) async fn follow_up(&mut self, message: &str) -> Result<(), XyDriverError> {
        self.agent.follow_up(message);
        Ok(())
    }

    pub(crate) async fn clear_queue(
        &mut self,
        clear_steer: bool,
        clear_follow_up: bool,
    ) -> Result<(), XyDriverError> {
        self.agent.clear_queues(clear_steer, clear_follow_up);
        Ok(())
    }

    pub(crate) fn queue_stats(&self) -> crate::agent::QueueStats {
        self.agent.queue_stats()
    }

    /// Execute a bash command (interactive bang / `Command::Bash`).
    ///
    /// c2760: output chunks are relayed to the injected
    /// [`crate::protocol::ports::BashOutputSink`] (non-journal session events);
    /// the finished result still returns synchronously.
    pub(crate) async fn execute_bash(
        &self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<XyBashResult, XyDriverError> {
        let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::user_bash(
            command,
            exclude_from_context,
            self.agent.cwd(),
        );
        self.agent.script_hook_cancel(ty, phase, ctx).await?;

        // Execution side generates the correlation id (start/done rows and
        // output chunks share it); the injected sink carries channel + cancel.
        let bash_id = uuid::Uuid::new_v4().to_string();
        let run = self
            .bash_run_sink
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();

        // Out-of-band kill switch (host `abort`); standalone callers (no sink)
        // fall back to a fresh token reachable only via `XyDriver::abort`.
        let cancel = run
            .as_ref()
            .map(|run| run.cancel.clone())
            .unwrap_or_default();

        // Relay raw output bytes to `BashChunk` events, reassembling UTF-8
        // sequences split across executor chunk boundaries.
        let (chunk_tx, chunk_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
        let relay =
            run.map(|run| tokio::spawn(relay_bash_chunks(bash_id.clone(), chunk_rx, run.tx)));

        let result = self
            .bang
            .execute(
                self.store.as_ref(),
                self.agent.session_id(),
                &bash_id,
                command,
                exclude_from_context,
                cancel,
                Some(chunk_tx),
                Some(self.agent.cwd().to_string()),
            )
            .await;
        if let Some(relay) = relay {
            let _ = relay.await;
        }
        result.map_err(XyDriverError::from)
    }
}

/// Relay executor output bytes to [`crate::protocol::ports::BashChunk`] events.
///
/// Executor chunks are raw 8 KiB reads, so a multi-byte UTF-8 sequence can be
/// split across a boundary; the incomplete tail is held back and prefixed to
/// the next chunk. `String::from_utf8_lossy` only runs on complete sequences
/// (invalid bytes → U+FFFD; a truncated sequence at stream end flushes as-is).
async fn relay_bash_chunks(
    bash_id: String,
    mut rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    tx: tokio::sync::mpsc::Sender<crate::protocol::ports::BashChunk>,
) {
    use crate::protocol::ports::BashChunk;

    let mut seq: u64 = 0;
    let mut pending: Vec<u8> = Vec::new();
    while let Some(bytes) = rx.recv().await {
        pending.extend_from_slice(&bytes);
        while !pending.is_empty() {
            let n = utf8_emit_len(&pending);
            if n == 0 {
                break;
            }
            let data = String::from_utf8_lossy(&pending[..n]).into_owned();
            pending.drain(..n);
            seq += 1;
            if tx
                .send(BashChunk {
                    bash_id: bash_id.clone(),
                    seq,
                    data,
                })
                .await
                .is_err()
            {
                return;
            }
        }
    }
    if !pending.is_empty() {
        seq += 1;
        let _ = tx
            .send(BashChunk {
                bash_id,
                seq,
                data: String::from_utf8_lossy(&pending).into_owned(),
            })
            .await;
    }
}

/// Length of the prefix of `buf` that is safe to convert now: the whole buffer
/// when it is valid UTF-8 or ends in invalid bytes (those emit as U+FFFD), or
/// everything before a truncated multi-byte tail (kept for the next chunk).
fn utf8_emit_len(buf: &[u8]) -> usize {
    match std::str::from_utf8(buf) {
        Ok(_) => buf.len(),
        Err(e) => match e.error_len() {
            None => e.valid_up_to(),
            Some(bad) => e.valid_up_to() + bad,
        },
    }
}

// ── Command executor (c2710): Command is the SSOT of session operations ────
#[async_trait]
impl crate::app::core::dispatch::SessionCommandExecutor for XyInProcessDriver {
    async fn execute_session_command(
        &mut self,
        cmd: crate::protocol::Command,
    ) -> Result<crate::app::core::dispatch::DispatchOutcome, XyDriverError> {
        use crate::app::core::dispatch::DispatchOutcome;
        use crate::protocol::Command;
        match cmd {
            Command::Abort { .. } => {
                crate::app::core::driver::XyDriver::abort(self);
                Ok(DispatchOutcome::Aborted { cancelled: true })
            }
            Command::GetState { .. } => Ok(DispatchOutcome::State(
                crate::app::core::driver::XyDriver::get_state(self),
            )),
            Command::SetModel { model_id, .. } => {
                let m = self.select_model(&model_id).await?;
                Ok(DispatchOutcome::Model(m))
            }
            Command::CycleModel { .. } => {
                let m = self.cycle_model().await?;
                Ok(DispatchOutcome::Model(m))
            }
            Command::GetAvailableModels { .. } => Ok(DispatchOutcome::Models(
                crate::app::core::driver::XyDriver::available_models(self),
            )),
            Command::SetThinkingLevel { level, .. } => {
                crate::app::core::dispatch::validate_nonempty_thinking_level(&level)?;
                self.set_thinking_level(level.clone()).await?;
                Ok(DispatchOutcome::ThinkingLevel(level))
            }
            Command::Bash {
                command,
                exclude_from_context,
                ..
            } => {
                let r = self.execute_bash(&command, exclude_from_context).await?;
                Ok(DispatchOutcome::Bash(r))
            }
            Command::Compact { instructions, .. } => {
                let did = self.compact(instructions).await?;
                Ok(DispatchOutcome::Compacted(did))
            }
            Command::GetSessionStats { .. } => {
                let stats = self.get_session_stats().await?;
                Ok(DispatchOutcome::SessionStats(serde_json::json!({
                    "session_id": stats.session_id,
                    "user_messages": stats.user_messages,
                    "assistant_messages": stats.assistant_messages,
                    "total_messages": stats.total_messages,
                    "thinking_level": stats.thinking_level,
                    "model": stats.model.map(|(p, m)| {
                        serde_json::json!({ "provider": p, "model_id": m })
                    }),
                })))
            }
            Command::ExportHtml { output_path, .. } => {
                let path = output_path
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::path::PathBuf::from("export.html"));
                let written = self.export_html(&path).await?;
                Ok(DispatchOutcome::ExportedPath(written))
            }
            Command::ExportJsonl { output_path, .. } => {
                let path = output_path
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::path::PathBuf::from("export.jsonl"));
                let written = self.export_jsonl(&path).await?;
                Ok(DispatchOutcome::ExportedPath(written))
            }
            Command::ImportJsonl { input_path, .. } => {
                let path = std::path::PathBuf::from(&input_path);
                let new_id = self.import_jsonl(&path).await?;
                Ok(DispatchOutcome::NewSession(new_id))
            }
            Command::SwitchSession { session_path, .. } => {
                // Derive session id from path (file stem).
                let new_id = std::path::Path::new(&session_path)
                    .file_stem()
                    .and_then(|st| st.to_str())
                    .unwrap_or(&session_path)
                    .to_string();
                let switched = self.switch_session(&new_id).await?;
                Ok(DispatchOutcome::SwitchedSession(switched))
            }
            Command::Fork {
                entry_id, position, ..
            } => {
                let pos = match position.as_deref() {
                    Some("before") => crate::protocol::session::ForkPosition::Before,
                    _ => crate::protocol::session::ForkPosition::At,
                };
                let new_id = self.fork_session(&entry_id, pos).await?;
                Ok(DispatchOutcome::NewSession(new_id))
            }
            Command::GetMessages { .. } => {
                let entries = self.get_messages().await?;
                let session_id =
                    crate::app::core::driver::XyDriver::session_id(self).unwrap_or_default();
                Ok(DispatchOutcome::Messages {
                    session_id,
                    entries,
                })
            }
            Command::SessionTree { kind, .. } => {
                Ok(DispatchOutcome::SessionTree(self.session_tree(kind).await?))
            }
            Command::TravelSessionTree { kind, entry_id, .. } => {
                Ok(DispatchOutcome::SessionTreeTravel(
                    self.travel_session_tree(kind, &entry_id).await?,
                ))
            }
            Command::AppendEntryLabel {
                target_id, label, ..
            } => {
                self.append_entry_label(&target_id, label.as_deref())
                    .await?;
                Ok(DispatchOutcome::Empty)
            }
            Command::ListSessions { .. } => {
                Ok(DispatchOutcome::Sessions(self.list_sessions().await?))
            }
            Command::LoadSessionEntries { session_id, .. } => Ok(DispatchOutcome::SessionEntries(
                self.load_session_entries(&session_id).await?,
            )),
            Command::NewSession { .. } => {
                Ok(DispatchOutcome::NewSession(self.new_session().await?))
            }
            Command::GetSessionName { .. } => {
                Ok(DispatchOutcome::SessionName(self.get_session_name().await?))
            }
            Command::SetSessionName { name, .. } => Ok(DispatchOutcome::SessionName(Some(
                self.set_session_name(&name).await?,
            ))),
            Command::SetSessionNameFor {
                session_id, name, ..
            } => Ok(DispatchOutcome::SessionName(Some(
                self.set_session_name_for(&session_id, &name).await?,
            ))),
            Command::DeleteSession { session_id, .. } => {
                self.delete_session(&session_id).await?;
                Ok(DispatchOutcome::Empty)
            }
            Command::Reload { .. } => Ok(DispatchOutcome::Reload(
                self.reload_runtime(&tokio_util::sync::CancellationToken::new())
                    .await?,
            )),
            Command::LoadedResources { .. } => Ok(DispatchOutcome::LoadedResources(
                crate::app::core::driver::XyDriver::loaded_resources_snapshot(self).await,
            )),
            Command::GetQueueStats { .. } => {
                let stats = self.queue_stats();
                Ok(DispatchOutcome::QueueStats {
                    steer_count: stats.steer_count,
                    follow_up_count: stats.follow_up_count,
                })
            }
            Command::GetCommands { .. } => Ok(DispatchOutcome::Commands(
                crate::app::core::driver::XyDriver::get_commands(self),
            )),
            Command::Steer { message, .. } => {
                self.steer(&message).await?;
                let stats = self.queue_stats();
                Ok(DispatchOutcome::QueueStats {
                    steer_count: stats.steer_count,
                    follow_up_count: stats.follow_up_count,
                })
            }
            Command::FollowUp { message, .. } => {
                self.follow_up(&message).await?;
                let stats = self.queue_stats();
                Ok(DispatchOutcome::QueueStats {
                    steer_count: stats.steer_count,
                    follow_up_count: stats.follow_up_count,
                })
            }
            Command::ClearQueue {
                clear_steer,
                clear_follow_up,
                ..
            } => {
                self.clear_queue(clear_steer, clear_follow_up).await?;
                let stats = self.queue_stats();
                Ok(DispatchOutcome::QueueStats {
                    steer_count: stats.steer_count,
                    follow_up_count: stats.follow_up_count,
                })
            }

            // These variants are the caller's responsibility (see dispatch module docs).
            Command::Prompt { .. }
            | Command::Quit { .. }
            | Command::Subscribe { .. }
            | Command::ApproveTool { .. }
            | Command::AnswerQuestion { .. } => {
                Err(crate::app::core::dispatch::transport_variant_error(&cmd))
            }
        }
    }
}
