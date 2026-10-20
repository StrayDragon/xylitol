//! Designing / in-process TUI: wrap the composition-root driver as the TUI crate's `XyDriver`.

use async_trait::async_trait;
use xylitol_app_tui::XyDriverError as TuiError;
use xylitol_app_tui::dispatch::{DispatchOutcome as TuiOutcome, SessionCommandExecutor};
use xylitol_app_tui::driver::{
    ClipboardCopyOutcome, CommandInfo, LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot,
    ModelInfo, ProjectTrustMode, ProjectTrustPersistReport, ReloadStepReport, RuntimeReloadReport,
    SessionState, XyDriver as TuiDriver,
};
use xylitol_protocol::Command;

use crate::wiring::dispatch::{
    DispatchOutcome as RootOutcome, SessionCommandExecutor as RootExecutor,
};
use crate::wiring::driver::XyDriverError as RootError;
use crate::wiring::driver::{
    ClipboardCopyOutcome as RootClip, CommandInfo as RootCmd, LoadedResourcesSnapshot as RootSnap,
    McpServerPhase as RootPhase, ModelInfo as RootModel, ProjectTrustMode as RootTrust,
    ProjectTrustPersistReport as RootTrustReport, RuntimeReloadReport as RootReload,
    SessionState as RootState, XyDriver as RootDriver, XyInProcessDriver,
};

pub(crate) struct TuiInProcessAdapter<'a> {
    inner: &'a mut XyInProcessDriver,
}

impl<'a> TuiInProcessAdapter<'a> {
    pub(crate) fn new(inner: &'a mut XyInProcessDriver) -> Self {
        Self { inner }
    }
}

fn map_err(e: RootError) -> TuiError {
    match e {
        RootError::NotFound { message, .. } => TuiError::not_found(message),
        RootError::Unsupported { message, .. } => TuiError::unsupported(message),
        RootError::InvalidInput { message, .. } => TuiError::invalid_input(message),
        RootError::Io { message, .. } => TuiError::io(message),
        RootError::Remote { message, .. } => TuiError::remote(message),
        RootError::Agent(err) => TuiError::Agent(err),
        RootError::Message { message, .. } => TuiError::message(message),
    }
}

fn map_model(m: RootModel) -> ModelInfo {
    ModelInfo {
        id: m.id,
        display_name: m.display_name,
        thinking: m.thinking,
        thinking_levels: m.thinking_levels,
        context_window: m.context_window,
    }
}

fn map_state(s: RootState) -> SessionState {
    SessionState {
        session_id: s.session_id,
        model: s.model.map(map_model),
        thinking_level: s.thinking_level,
        leaf_entry_id: s.leaf_entry_id,
    }
}

fn map_cmd(c: RootCmd) -> CommandInfo {
    CommandInfo {
        name: c.name,
        description: c.description,
    }
}

fn map_reload(r: RootReload) -> RuntimeReloadReport {
    RuntimeReloadReport {
        steps: r
            .steps
            .into_iter()
            .map(|s| ReloadStepReport {
                step: s.step,
                ok: s.ok,
                message: s.message,
            })
            .collect(),
        cancelled: r.cancelled,
    }
}

fn map_snap(s: RootSnap) -> LoadedResourcesSnapshot {
    LoadedResourcesSnapshot {
        skill_names: s.skill_names,
        mcp_connected: s.mcp_connected,
        mcp_configured: s.mcp_configured,
        mcp_diag_short: s.mcp_diag_short,
        mcp_connecting_label: s.mcp_connecting_label,
        mcp_servers: s
            .mcp_servers
            .into_iter()
            .map(|row| McpServerSnapshot {
                id: row.id,
                phase: match row.phase {
                    RootPhase::Connecting => McpServerPhase::Connecting,
                    RootPhase::Connected => McpServerPhase::Connected,
                    RootPhase::Failed => McpServerPhase::Failed,
                },
                tools_armed: row.tools_armed,
                tool_count: row.tool_count,
            })
            .collect(),
        mcp_bootstrap_complete: s.mcp_bootstrap_complete,
        tools_table_frozen: s.tools_table_frozen,
        mcp_gate_notice: s.mcp_gate_notice,
        obs_diag: s.obs_diag,
    }
}

fn map_outcome(o: RootOutcome) -> TuiOutcome {
    match o {
        RootOutcome::Aborted { cancelled } => TuiOutcome::Aborted { cancelled },
        RootOutcome::State(s) => TuiOutcome::State(map_state(s)),
        RootOutcome::Model(m) => TuiOutcome::Model(map_model(m)),
        RootOutcome::Models(v) => TuiOutcome::Models(v.into_iter().map(map_model).collect()),
        RootOutcome::ThinkingLevel(s) => TuiOutcome::ThinkingLevel(s),
        RootOutcome::Bash(b) => TuiOutcome::Bash(b),
        RootOutcome::Compacted(b) => TuiOutcome::Compacted(b),
        RootOutcome::SessionStats(v) => TuiOutcome::SessionStats(v),
        RootOutcome::ExportedPath(p) => TuiOutcome::ExportedPath(p),
        RootOutcome::NewSession(s) => TuiOutcome::NewSession(s),
        RootOutcome::SwitchedSession(s) => TuiOutcome::SwitchedSession(s),
        RootOutcome::Messages {
            session_id,
            entries,
        } => TuiOutcome::Messages {
            session_id,
            entries,
        },
        RootOutcome::EstimateContext(e) => TuiOutcome::EstimateContext(e),
        RootOutcome::SessionTree(t) => TuiOutcome::SessionTree(t),
        RootOutcome::SessionTreeTravel(t) => TuiOutcome::SessionTreeTravel(t),
        RootOutcome::Sessions(v) => TuiOutcome::Sessions(v),
        RootOutcome::SessionEntries(v) => TuiOutcome::SessionEntries(v),
        RootOutcome::SessionName(n) => TuiOutcome::SessionName(n),
        RootOutcome::Reload(r) => TuiOutcome::Reload(map_reload(r)),
        RootOutcome::LoadedResources(s) => TuiOutcome::LoadedResources(map_snap(s)),
        RootOutcome::Empty => TuiOutcome::Empty,
        RootOutcome::Commands(v) => TuiOutcome::Commands(v.into_iter().map(map_cmd).collect()),
        RootOutcome::QueueStats {
            steer_count,
            follow_up_count,
        } => TuiOutcome::QueueStats {
            steer_count,
            follow_up_count,
        },
    }
}

fn map_trust(mode: ProjectTrustMode) -> RootTrust {
    match mode {
        ProjectTrustMode::TrustCwd => RootTrust::TrustCwd,
        ProjectTrustMode::TrustParent => RootTrust::TrustParent,
        ProjectTrustMode::Deny => RootTrust::Deny,
    }
}

#[async_trait]
impl SessionCommandExecutor for TuiInProcessAdapter<'_> {
    async fn execute_session_command(&mut self, cmd: Command) -> Result<TuiOutcome, TuiError> {
        RootExecutor::execute_session_command(self.inner, cmd)
            .await
            .map(map_outcome)
            .map_err(map_err)
    }
}

#[async_trait]
impl TuiDriver for TuiInProcessAdapter<'_> {
    async fn run(&mut self, prompt: &str) -> xylitol_app_tui::driver::EventStream {
        RootDriver::run(self.inner, prompt).await
    }

    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<xylitol_app_tui::driver::BashRun, TuiError> {
        let fut = RootDriver::bash_run(self.inner, command, exclude_from_context)
            .await
            .map_err(map_err)?;
        Ok(Box::pin(async move { fut.await.map_err(map_err) }))
    }

    fn abort(&self) {
        RootDriver::abort(self.inner);
    }

    fn current_model(&self) -> Option<ModelInfo> {
        RootDriver::current_model(self.inner).map(map_model)
    }

    fn active_turn(&self) -> Option<(String, String, bool)> {
        RootDriver::active_turn(self.inner)
    }

    fn has_active_turn(&self) -> bool {
        RootDriver::has_active_turn(self.inner)
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        RootDriver::available_models(self.inner)
            .into_iter()
            .map(map_model)
            .collect()
    }

    fn thinking_level(&self) -> String {
        RootDriver::thinking_level(self.inner)
    }

    fn session_id(&self) -> Option<String> {
        RootDriver::session_id(self.inner)
    }

    fn set_bash_run_sink(&mut self, sink: Option<xylitol_plugin_sdk::BashOutputSink>) {
        RootDriver::set_bash_run_sink(self.inner, sink);
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, TuiError> {
        RootDriver::estimate_context_tokens(self.inner)
            .await
            .map_err(map_err)
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        RootDriver::get_commands(self.inner)
            .into_iter()
            .map(map_cmd)
            .collect()
    }

    fn leaf_entry_id(&self) -> Option<String> {
        RootDriver::leaf_entry_id(self.inner)
    }

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        RootDriver::dollar_skill_catalog(self.inner)
    }

    fn session_store(&self) -> Option<std::sync::Arc<dyn xylitol_plugin_sdk::XySessionStore>> {
        RootDriver::session_store(self.inner)
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        map_snap(RootDriver::loaded_resources_snapshot(self.inner).await)
    }

    fn mcp_blocks_agent(&self) -> bool {
        RootDriver::mcp_blocks_agent(self.inner)
    }

    fn is_tools_frozen(&self) -> bool {
        RootDriver::is_tools_frozen(self.inner)
    }

    async fn arm_tool_freeze_gate(&mut self) {
        RootDriver::arm_tool_freeze_gate(self.inner).await;
    }

    fn take_mcp_gate_notice(&mut self) -> Option<String> {
        RootDriver::take_mcp_gate_notice(self.inner)
    }

    async fn begin_mcp_bootstrap(&mut self) {
        RootDriver::begin_mcp_bootstrap(self.inner).await;
    }

    async fn poll_mcp_bootstrap(&mut self) -> bool {
        RootDriver::poll_mcp_bootstrap(self.inner).await
    }

    async fn reload_runtime(
        &mut self,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<RuntimeReloadReport, TuiError> {
        RootDriver::reload_runtime(self.inner, cancel)
            .await
            .map(map_reload)
            .map_err(map_err)
    }

    async fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, TuiError> {
        RootDriver::persist_project_trust(self.inner, map_trust(mode))
            .await
            .map(|r: RootTrustReport| ProjectTrustPersistReport {
                trusted: r.trusted,
                saved_path: r.saved_path,
                message: r.message,
            })
            .map_err(map_err)
    }

    async fn copy_text_to_clipboard(
        &mut self,
        text: &str,
    ) -> Result<ClipboardCopyOutcome, TuiError> {
        RootDriver::copy_text_to_clipboard(self.inner, text)
            .await
            .map(|r: RootClip| ClipboardCopyOutcome {
                pending_osc52: r.pending_osc52,
            })
            .map_err(map_err)
    }

    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, TuiError> {
        RootDriver::stage_clipboard_image(self.inner)
            .await
            .map_err(map_err)
    }

    async fn read_clipboard_text(&mut self) -> Result<Option<String>, TuiError> {
        RootDriver::read_clipboard_text(self.inner)
            .await
            .map_err(map_err)
    }
}
