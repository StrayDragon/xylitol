//! Shared agent construction for all application entry points.

use std::sync::Arc;

use crate::agent::AgentBuilder;
use crate::agent::ReActAgent;
use crate::agent::compaction::CompactionSettings;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::session::QueueMode;
use crate::agent::tools::ToolSet;
use crate::infra::bash_exec::InfraBashExecutor;
use crate::infra::event::EventBus;
use crate::infra::export::StdExportIo;
use crate::infra::permission;
use crate::infra::session::SessionManager;
use crate::runtime_protocol::{
    XyBashExecutor, XyEventSink, XyExportIo, XyModelBuilder, XyPermission, XySessionStore,
};

/// Options for [`build_agent`].
pub struct BuildAgentOptions {
    pub model_registry: ModelRegistry,
    pub system_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub append_system_prompt: Vec<String>,
    pub max_iterations: u32,
    pub compaction_threshold: f64,
    pub cwd: String,
    pub compaction_settings: Option<CompactionSettings>,
    pub permission: Option<Arc<dyn XyPermission>>,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
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
            max_iterations: 50,
            compaction_threshold: 0.8,
            cwd: ".".into(),
            compaction_settings: None,
            permission: None,
            steering_mode: QueueMode::default(),
            follow_up_mode: QueueMode::default(),
        }
    }
}

/// Construct a fully-wired [`ReActAgent`] from the given options.
///
/// This is the single composition-root helper used by CLI, RPC, server, and
/// future TUI/GUI modes. It injects the concrete infra implementations
/// (`SessionManager`, `EventBus`, `InfraBashExecutor`, `StdExportIo`) into the
/// agent without letting `agent/` know about `infra/` types.
pub fn build_agent(options: BuildAgentOptions) -> Result<ReActAgent, String> {
    let sessions_dir = SessionManager::default_dir();
    std::fs::create_dir_all(&sessions_dir).map_err(|e| format!("create sessions dir: {e}"))?;
    let session_mgr = SessionManager::new(sessions_dir);

    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(EventBus::new());
    let bash_executor: Arc<dyn XyBashExecutor> = Arc::new(InfraBashExecutor::new());
    let export_io: Arc<dyn XyExportIo> = Arc::new(StdExportIo::new());

    let model_builder: XyModelBuilder = Arc::new(crate::infra::provider::factory::build_provider);
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
    .max_iterations(options.max_iterations)
    .compaction_threshold(options.compaction_threshold)
    .compaction_settings(options.compaction_settings)
    .cwd(options.cwd)
    .bash(bash_executor)
    .export_io(export_io)
    .steering_mode(options.steering_mode)
    .follow_up_mode(options.follow_up_mode);

    if let Some(sp) = options.system_prompt {
        builder = builder.system_prompt(sp);
    }

    builder.build()
}

/// Reload MCP tools onto an agent (composition-root seam; may use infra).
///
/// - Empty / missing servers → no manager constructed; ToolSet = builtins only.
/// - Non-empty → connect, discover, merge `mcp:*` tools; store manager in `mcp`.
///
/// Takes effect on the next agent `run`. Shuts down any previous manager in `mcp`.
pub async fn reload_mcp_tools(
    agent: &mut ReActAgent,
    mcp: &mut Option<Arc<crate::infra::mcp::McpClientManager>>,
    servers: &[crate::infra::mcp::McpServerConfig],
) -> Result<(), String> {
    use crate::infra::mcp::{connect_and_discover, mcp_enabled};

    if let Some(old) = mcp.take() {
        old.shutdown().await;
    }

    let mut tools = ToolSet::from_iter(crate::infra::tools::default_tools());
    if mcp_enabled(&Some(servers.to_vec()))
        && let Some((manager, mcp_tools)) = connect_and_discover(servers).await?
    {
        tools = tools.merge(ToolSet::from_iter(mcp_tools));
        *mcp = Some(manager);
    }

    agent.set_tools(tools);
    Ok(())
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
        let mut driver = crate::app::core::driver::InProcessDriver::new(agent, store);
        let mut mcp = None;
        reload_mcp_tools(driver.agent_mut(), &mut mcp, &[])
            .await
            .unwrap();
        assert!(mcp.is_none());
        assert!(!mcp_enabled(&Some(vec![])));
        let names: Vec<_> = driver
            .agent_mut()
            .inner()
            .tools()
            .iter()
            .map(|t| t.name().to_string())
            .collect();
        assert!(names.iter().all(|n| !n.starts_with("mcp:")));
        assert!(names.iter().any(|n| n == "read"));
    }
}
