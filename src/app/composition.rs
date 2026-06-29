//! Shared agent construction for all application entry points.

use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use crate::agent::facade::Agent;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::tools::ToolRegistry;
use crate::infra::bash_exec::InfraBashExecutor;
use crate::infra::event::EventBus;
use crate::infra::export::StdExportIo;
use crate::infra::sandbox;
use crate::infra::session::SessionManager;
use crate::runtime_protocol::{
    BashExecutor, EventSink, ExportIo, ModelBuilder, SandboxEngine, SessionStore,
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
    pub sandbox_engine: Option<Arc<dyn SandboxEngine>>,
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
            sandbox_engine: None,
        }
    }
}

/// Construct a fully-wired [`Agent`] from the given options.
///
/// This is the single composition-root helper used by CLI, RPC, server, and
/// future TUI/GUI modes. It injects the concrete infra implementations
/// (`SessionManager`, `EventBus`, `InfraBashExecutor`, `StdExportIo`) into the
/// agent without letting `agent/` know about `infra/` types (HC-1/HC-2).
pub fn build_agent(options: BuildAgentOptions) -> Result<Agent, String> {
    let tool_registry = ToolRegistry::from_tools(crate::infra::tools::default_tools());
    let sessions_dir = SessionManager::default_dir();
    std::fs::create_dir_all(&sessions_dir).map_err(|e| format!("create sessions dir: {e}"))?;
    let session_mgr = SessionManager::new(sessions_dir);

    let store: Arc<dyn SessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn EventSink> = Arc::new(EventBus::new());
    let bash_executor: Arc<dyn BashExecutor> = Arc::new(InfraBashExecutor::new());
    let export_io: Arc<dyn ExportIo> = Arc::new(StdExportIo::new());

    let model_builder: ModelBuilder = Arc::new(crate::infra::provider::factory::build_provider);
    let sandbox = options.sandbox_engine.unwrap_or_else(sandbox::noop_engine);

    let agent = Agent::with_ports(
        options.model_registry,
        tool_registry,
        store,
        sink,
        options.system_prompt,
        options.context_files,
        options.append_system_prompt,
        options.max_iterations,
        options.compaction_threshold,
        options.cwd,
        options.compaction_settings,
        model_builder,
        sandbox,
        bash_executor,
        export_io,
    );

    Ok(agent)
}
