//! Shared agent construction for all application entry points.

use std::sync::Arc;

use crate::agent::AgentBuilder;
use crate::agent::AgentRuntime;
use crate::agent::compaction::CompactionSettings;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::session::QueueMode;
use crate::agent::tools::ToolSet;
use crate::app::core::driver_error::XyDriverError;
use crate::infra::bash_exec::InfraBashExecutor;
use crate::infra::config::types::HooksConfig;
use crate::infra::event::EventBus;
use crate::infra::export::StdExportIo;
use crate::infra::hooks::HookDispatcher;
use crate::infra::permission;
use crate::infra::session::SessionManager;
use crate::protocol::ports::{
    XyBashExecutor, XyBatchMode, XyEventSink, XyExportIo, XyHookBus, XyModelBuilder, XyPermission,
    XySessionStore,
};

pub use crate::app::core::mcp_spec::{McpServerSpec, McpTransportSpec};

/// Options for [`build_agent`].
pub struct BuildAgentOptions {
    pub model_registry: ModelRegistry,
    pub system_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub append_system_prompt: Vec<String>,
    /// Skills catalog for `<available_skills>` (c1085).
    pub skills: Vec<crate::protocol::resource::SkillInfo>,
    pub cwd: String,
    pub compaction_settings: Option<CompactionSettings>,
    pub permission: Option<Arc<dyn XyPermission>>,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
    /// Optional lifecycle sink (compaction etc.). Default: in-process [`EventBus`].
    /// Turn UX still uses the `XyDriver::run` EventStream, not this bus.
    pub event_sink: Option<Arc<dyn XyEventSink>>,
    /// Three-tier script hook configuration (empty = zero-cost no-op).
    pub hooks_config: HooksConfig,
    /// Same-turn tool batch mode from `AppConfig.tool_batch` (c1545).
    pub batch_mode: XyBatchMode,
}

impl Default for BuildAgentOptions {
    fn default() -> Self {
        Self {
            model_registry: ModelRegistry::new(Arc::new(
                crate::infra::config::value::InfraSecretResolver::new(),
            )),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            skills: Vec::new(),
            cwd: ".".into(),
            compaction_settings: None,
            permission: None,
            steering_mode: QueueMode::default(),
            follow_up_mode: QueueMode::default(),
            event_sink: None,
            hooks_config: HooksConfig::default(),
            batch_mode: XyBatchMode::Sequential,
        }
    }
}

/// Construct a fully-wired [`AgentRuntime`] from the given options.
///
/// This is the single composition-root helper used by CLI, RPC, server, and
/// future TUI/GUI modes. It injects the concrete infra implementations
/// (`SessionManager`, `XyEventSink`, `InfraBashExecutor`, `StdExportIo`) into the
/// agent without letting `agent/` know about `infra/` types.
///
/// **Event paths:** turn progress is the `XyDriver::run` → `XyEvent` stream.
/// The injected [`XyEventSink`] (default [`EventBus`]) is for side lifecycle
/// (e.g. compaction); it is not the multi-client turn bus.
pub fn build_agent(options: BuildAgentOptions) -> Result<AgentRuntime, XyDriverError> {
    let sessions_dir = SessionManager::default_dir();
    std::fs::create_dir_all(&sessions_dir).map_err(|e| format!("create sessions dir: {e}"))?;
    let session_mgr = SessionManager::new(sessions_dir);

    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = options
        .event_sink
        .unwrap_or_else(|| Arc::new(EventBus::new()));
    let bash_executor: Arc<dyn XyBashExecutor> = Arc::new(InfraBashExecutor::new());
    let export_io: Arc<dyn XyExportIo> = Arc::new(StdExportIo::new());

    let hook_dispatcher = Arc::new(HookDispatcher::new(&options.hooks_config));
    let hooks_for_provider = if hook_dispatcher.is_empty() {
        None
    } else {
        Some(hook_dispatcher.clone())
    };
    let hook_bus: Option<Arc<dyn XyHookBus>> = if hook_dispatcher.is_empty() {
        None
    } else {
        Some(hook_dispatcher)
    };

    let model_builder: XyModelBuilder = {
        let hooks = hooks_for_provider;
        Arc::new(move |config| {
            crate::infra::provider::factory::build_provider_with_hooks(config, hooks.clone())
        })
    };
    let permission = options
        .permission
        .unwrap_or_else(permission::allow_all_permission);

    let mut builder = AgentBuilder::new(
        options.model_registry,
        model_builder,
        store,
        sink,
        permission,
    )
    .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
    .context_files(options.context_files)
    .append_system_prompt(options.append_system_prompt)
    .skills(options.skills)
    .compaction_settings(options.compaction_settings)
    .cwd(options.cwd)
    .bash(bash_executor)
    .export_io(export_io)
    .steering_mode(options.steering_mode)
    .follow_up_mode(options.follow_up_mode)
    .hook_bus(hook_bus)
    .batch_mode(options.batch_mode);

    if let Some(sp) = options.system_prompt {
        builder = builder.system_prompt(sp);
    }

    builder.build().map_err(XyDriverError::from)
}

/// Owns MCP client connections for a local XyDriver session (composition seam).
///
/// Held by the surface (cli/server) so connections stay alive across turns and
/// can be shut down / replaced on reload without `XyDriver` importing infra.
pub struct McpSession {
    manager: Option<Arc<crate::infra::mcp::McpClientManager>>,
}

impl Default for McpSession {
    fn default() -> Self {
        Self::new()
    }
}

impl McpSession {
    pub fn new() -> Self {
        Self { manager: None }
    }

    #[cfg(test)]
    pub fn has_manager(&self) -> bool {
        self.manager.is_some()
    }

    /// Read-only connected MCP snapshot (c1080 / mcp5). Empty when no manager.
    pub async fn connected_servers(&self) -> Vec<crate::infra::mcp::ConnectedMcpServer> {
        match &self.manager {
            Some(m) => m.connected_servers().await,
            None => Vec::new(),
        }
    }

    /// Diagnostics from the last connect/reload (c1080 / mcp4).
    pub async fn diagnostics(&self) -> Vec<crate::infra::mcp::McpConnectDiagnostic> {
        match &self.manager {
            Some(m) => m.diagnostics().await,
            None => Vec::new(),
        }
    }

    /// Install a freshly discovered manager (c1200). Caller updates driver tools separately
    /// when borrow-split is required.
    pub fn set_manager(&mut self, manager: Arc<crate::infra::mcp::McpClientManager>) {
        self.manager = Some(manager);
    }

    pub fn take_manager(&mut self) -> Option<Arc<crate::infra::mcp::McpClientManager>> {
        self.manager.take()
    }

    /// Reload MCP tools onto `driver` (empty servers → builtins only, zero-cost).
    pub async fn reload(
        &mut self,
        driver: &mut crate::app::core::driver::XyInProcessDriver,
        servers: &[McpServerSpec],
    ) -> Result<(), XyDriverError> {
        use crate::infra::mcp::{connect_and_discover, mcp_enabled};

        if let Some(old) = self.manager.take() {
            old.shutdown().await;
        }

        let infra = McpServerSpec::to_infra_list(servers);
        let mut tools = ToolSet::from_iter(crate::infra::tools::default_tools());
        if mcp_enabled(&Some(infra.clone()))
            && let Some((manager, mcp_tools)) = connect_and_discover(&infra).await?
        {
            tools = tools.merge(ToolSet::from_iter(mcp_tools));
            self.manager = Some(manager);
        }

        driver.set_tools(tools);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::mcp::mcp_enabled;

    #[tokio::test]
    async fn reload_empty_is_zero_cost_no_manager() {
        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let mut driver = crate::app::core::driver::XyInProcessDriver::new(agent, store);
        let mut mcp = McpSession::new();
        mcp.reload(&mut driver, &[]).await.unwrap();
        assert!(!mcp.has_manager());
        assert!(!mcp_enabled(&Some(vec![])));
        let names: Vec<_> = driver.tool_names_for_test();
        assert!(names.iter().all(|n| !n.starts_with("mcp:")));
        assert!(names.iter().any(|n| n == "read"));
        assert!(mcp.connected_servers().await.is_empty());
        assert!(mcp.diagnostics().await.is_empty());
    }
}
