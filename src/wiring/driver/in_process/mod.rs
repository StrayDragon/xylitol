//! Product in-process driver: runtime core + official plugin injection + MCP reload.

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use async_trait::async_trait;

use xylitol_plugin_sdk::XySessionStore;
use xylitol_runtime::agent::AgentRuntime;
use xylitol_runtime::{
    BangExecHandler, SessionExporter, XyDriver, XyDriverError, XyInProcessDriver as RuntimeDriver,
};

use super::{
    ClipboardCopyOutcome, CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo,
    ProjectTrustMode, ProjectTrustPersistReport, RuntimeReloadReport,
};

mod mcp;
mod reload;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use xylitol_runtime::driver::{relay_bash_chunks, utf8_emit_len};

use mcp::McpBootState;
use mcp::McpDiscoverOk;

struct InProcessReloadState {
    cwd: std::path::PathBuf,
    agent_dir: std::path::PathBuf,
    project_trusted: bool,
    mcp: crate::wiring::composition::McpSession,
    mcp_servers: Vec<crate::wiring::mcp_spec::McpServerSpec>,
}

/// Composition-root driver: runtime core plus MCP / plugin injection.
pub struct XyInProcessDriver {
    pub inner: RuntimeDriver,
    todo: Arc<xylitol_official_plugin_tools::SessionAgentTodoGateway>,
    reload: Option<InProcessReloadState>,
    mcp_boot: McpBootState,
    late_mcp_discover: Option<tokio::task::JoinHandle<McpDiscoverOk>>,
    mcp_gate_notice: Option<String>,
    tool_gate_deadline: Option<std::time::Instant>,
}

impl Deref for XyInProcessDriver {
    type Target = RuntimeDriver;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for XyInProcessDriver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl XyInProcessDriver {
    /// Construct from a built agent plus the store used to build it.
    pub fn new(agent: AgentRuntime, store: Arc<dyn XySessionStore>) -> Self {
        Self::with_surface_ops(
            agent,
            store,
            BangExecHandler::new(Some(Arc::new(
                xylitol_official_plugin_tools::bash_exec::InfraBashExecutor::new(),
            ))),
            SessionExporter::new(Some(Arc::new(
                crate::wiring::infra::export::StdExportIo::new(),
            ))),
        )
    }

    /// Construct with explicit bang + export collaborators (tests / embed).
    pub fn with_surface_ops(
        agent: AgentRuntime,
        store: Arc<dyn XySessionStore>,
        bang: BangExecHandler,
        exporter: SessionExporter,
    ) -> Self {
        let todo = xylitol_official_plugin_tools::SessionAgentTodoGateway::new(store.clone());
        let mut inner = RuntimeDriver::with_surface_ops(agent, store, bang, exporter);
        inner.set_todo_gateway(todo.clone());
        inner.set_tools(xylitol_runtime::agent::tools::ToolSet::from_iter(
            xylitol_official_plugin_tools::default_tools_with_todo(todo.clone()),
        ));
        inner.set_slash_commands(
            crate::wiring::product_commands::product_slash_commands()
                .into_iter()
                .map(|c| CommandInfo {
                    name: c.name.to_string(),
                    description: c.description.to_string(),
                })
                .collect(),
        );
        Self {
            inner,
            todo,
            reload: None,
            mcp_boot: McpBootState::Idle,
            late_mcp_discover: None,
            mcp_gate_notice: None,
            tool_gate_deadline: None,
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
            SessionExporter::new(Some(Arc::new(
                crate::wiring::infra::export::StdExportIo::new(),
            ))),
        )
    }

    /// Install TUI-only `ask` tool and remember the gateway for MCP reload.
    pub fn install_ask_tool(&mut self, gateway: Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>) {
        self.inner.install_ask_tool(gateway.clone());
        self.inner
            .set_tools(xylitol_runtime::agent::tools::ToolSet::from_iter(
                xylitol_official_plugin_tools::default_tools_with_ask_and_todo(
                    gateway,
                    self.todo.clone(),
                ),
            ));
    }

    /// Builtins for ToolSet rebuild: store-bound Todo (+ optional ask).
    pub fn builtins_for_reload(&self) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
        match self.inner.ask_gateway() {
            Some(g) => {
                xylitol_official_plugin_tools::default_tools_with_ask_and_todo(g, self.todo.clone())
            }
            None => xylitol_official_plugin_tools::default_tools_with_todo(self.todo.clone()),
        }
    }

    /// Store-bound Todo gateway (shared with `todo_*` builtins).
    pub fn todo_gateway(&self) -> Arc<xylitol_official_plugin_tools::SessionAgentTodoGateway> {
        self.todo.clone()
    }

    /// Enable `/reload` and MCP ownership for the process lifetime (c1120).
    pub fn enable_reload_state(
        &mut self,
        cwd: std::path::PathBuf,
        agent_dir: std::path::PathBuf,
        project_trusted: bool,
        mcp_servers: Vec<crate::wiring::mcp_spec::McpServerSpec>,
    ) {
        self.reload = Some(InProcessReloadState {
            cwd,
            agent_dir,
            project_trusted,
            mcp: crate::wiring::composition::McpSession::new(),
            mcp_servers,
        });
    }
}

#[async_trait]
impl XyDriver for XyInProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        self.ensure_tool_table_frozen().await;
        XyDriver::run(&mut self.inner, prompt).await
    }

    fn abort(&self) {
        XyDriver::abort(&self.inner);
    }

    fn current_model(&self) -> Option<ModelInfo> {
        XyDriver::current_model(&self.inner)
    }

    fn active_turn(&self) -> Option<(String, String, bool)> {
        XyDriver::active_turn(&self.inner)
    }

    fn has_active_turn(&self) -> bool {
        XyDriver::has_active_turn(&self.inner)
    }

    fn available_models(&self) -> Vec<ModelInfo> {
        XyDriver::available_models(&self.inner)
    }

    fn thinking_level(&self) -> String {
        XyDriver::thinking_level(&self.inner)
    }

    fn session_id(&self) -> Option<String> {
        XyDriver::session_id(&self.inner)
    }

    fn set_bash_run_sink(&mut self, sink: Option<xylitol_plugin_sdk::BashOutputSink>) {
        XyDriver::set_bash_run_sink(&mut self.inner, sink);
    }

    async fn estimate_context_tokens(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
        XyDriver::estimate_context_tokens(&self.inner).await
    }

    fn get_commands(&self) -> Vec<CommandInfo> {
        XyDriver::get_commands(&self.inner)
    }

    fn leaf_entry_id(&self) -> Option<String> {
        XyDriver::leaf_entry_id(&self.inner)
    }

    fn session_store(&self) -> Option<Arc<dyn XySessionStore>> {
        XyDriver::session_store(&self.inner)
    }

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        XyDriver::dollar_skill_catalog(&self.inner)
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        XyInProcessDriver::loaded_resources_snapshot(self).await
    }

    fn mcp_blocks_agent(&self) -> bool {
        XyInProcessDriver::mcp_blocks_agent(self)
    }

    fn is_tools_frozen(&self) -> bool {
        XyDriver::is_tools_frozen(&self.inner)
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

    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<xylitol_runtime::BashRun, XyDriverError> {
        XyDriver::bash_run(&mut self.inner, command, exclude_from_context).await
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
