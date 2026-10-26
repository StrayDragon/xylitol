use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::ask::AskUserGateway;
use xylitol_protocol::Command;
use xylitol_runtime::XyDriver;
use xylitol_runtime::XyDriverError;
use xylitol_runtime::agent::RuntimeBaseline;
use xylitol_runtime::driver::{
    CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, RuntimeReloadReport,
};
use xylitol_serve::occupancy::{OccupancyEngine, OccupancyFactory};

use crate::composition::build_baseline_with_store;
use crate::dispatch::{SessionCommandExecutor, outcome_to_value};
use crate::driver::{XyInProcessDriver, assemble_driver};
use crate::mcp_spec::McpServerSpec;
use crate::occupancy::host::{HostState, ReloadBaseline};

pub struct ProductOccupancyFactory {
    ports: RuntimeBaseline,
    cwd: PathBuf,
    agent_dir: PathBuf,
    project_trusted: bool,
    mcp_servers: Vec<McpServerSpec>,
    default_model_id: Option<String>,
    ctx: Option<xylitol_plugin_sdk::rt::Ctx>,
    _loader: Option<rutis_loader::Loader>,
}

impl ProductOccupancyFactory {
    pub fn new(
        ports: RuntimeBaseline,
        cwd: PathBuf,
        agent_dir: PathBuf,
        project_trusted: bool,
        mcp_servers: Vec<McpServerSpec>,
        default_model_id: Option<String>,
    ) -> Self {
        Self {
            ports,
            cwd,
            agent_dir,
            project_trusted,
            mcp_servers,
            default_model_id,
            ctx: None,
            _loader: None,
        }
    }

    pub fn with_prepare_ctx(
        mut self,
        ctx: xylitol_plugin_sdk::rt::Ctx,
        loader: rutis_loader::Loader,
    ) -> Self {
        self.ctx = Some(ctx);
        self._loader = Some(loader);
        self
    }
}

impl OccupancyFactory for ProductOccupancyFactory {
    fn new_driver_at(&self, workspace: &Path) -> Box<dyn OccupancyEngine> {
        let agent = self.ports.materialize_runtime();
        let mut driver = assemble_driver(agent, self.ports.store.clone());
        driver.enable_reload_state(
            workspace.to_path_buf(),
            self.agent_dir.clone(),
            self.project_trusted(workspace),
        );
        if let Some(ctx) = &self.ctx {
            driver.attach_ctx(ctx.clone());
            if let Some(hub) = ctx.get::<dyn xylitol_plugin_sdk::SessionPrepareHub>() {
                driver.set_prepare_hub(hub);
            }
        } else if !self.mcp_servers.is_empty() {
            let servers = crate::mcp_spec::McpServerSpec::to_infra_list(&self.mcp_servers);
            driver.set_prepare_hub(std::sync::Arc::new(
                xylitol_official_plugin_mcp::McpPrepareHub::new(servers),
            ));
        }
        driver
            .agent
            .set_cwd(workspace.to_string_lossy().into_owned());
        Box::new(OccupancyDriver { inner: driver })
    }

    fn project_trusted(&self, workspace: &Path) -> bool {
        let _ = workspace;
        self.project_trusted
    }

    fn default_model_id(&self) -> Option<String> {
        self.default_model_id.clone()
    }

    fn cwd(&self) -> PathBuf {
        self.cwd.clone()
    }

    fn agent_dir(&self) -> PathBuf {
        self.agent_dir.clone()
    }
}

struct OccupancyDriver {
    inner: XyInProcessDriver,
}

#[async_trait]
impl XyDriver for OccupancyDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        XyDriver::run(&mut self.inner, prompt).await
    }
    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<xylitol_runtime::BashRun, XyDriverError> {
        XyDriver::bash_run(&mut self.inner, command, exclude_from_context).await
    }
    fn abort(&self) {
        XyDriver::abort(&self.inner)
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
        XyDriver::set_bash_run_sink(&mut self.inner, sink)
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
    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        XyDriver::dollar_skill_catalog(&self.inner)
    }
    fn session_store(&self) -> Option<Arc<dyn XySessionStore>> {
        XyDriver::session_store(&self.inner)
    }
    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
        XyDriver::loaded_resources_snapshot(&self.inner).await
    }
    fn mcp_blocks_agent(&self) -> bool {
        XyDriver::mcp_blocks_agent(&self.inner)
    }
    fn is_tools_frozen(&self) -> bool {
        XyDriver::is_tools_frozen(&self.inner)
    }
    async fn arm_tool_freeze_gate(&mut self) {
        XyDriver::arm_tool_freeze_gate(&mut self.inner).await
    }
    fn take_mcp_gate_notice(&mut self) -> Option<String> {
        XyDriver::take_mcp_gate_notice(&mut self.inner)
    }
    async fn begin_session_prepare(&mut self) {
        XyDriver::begin_session_prepare(&mut self.inner).await
    }
    async fn poll_prepare_progress(&mut self) -> bool {
        XyDriver::poll_prepare_progress(&mut self.inner).await
    }
    async fn wait_session_prepare(&mut self) {
        XyDriver::wait_session_prepare(&mut self.inner).await
    }
    async fn reload_runtime(
        &mut self,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        XyDriver::reload_runtime(&mut self.inner, cancel).await
    }
    async fn persist_project_trust(
        &mut self,
        mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        XyDriver::persist_project_trust(&mut self.inner, mode).await
    }
}

#[async_trait]
impl OccupancyEngine for OccupancyDriver {
    fn install_ask_tool(&mut self, gateway: Arc<dyn AskUserGateway>) {
        crate::driver::install_product_ask_tool(&mut self.inner, gateway);
    }

    fn bind_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        self.inner
            .agent
            .bind_session(session_id.to_string())
            .map_err(XyDriverError::from)
    }

    async fn restore_model(&mut self, model_id: &str) -> Result<(), XyDriverError> {
        XyInProcessDriver::restore_model(&mut self.inner, model_id).await
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        XyInProcessDriver::switch_session(&mut self.inner, session_id).await
    }

    async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError> {
        XyInProcessDriver::select_model(&mut self.inner, model_id).await
    }

    fn model_provider(&self, model_id: &str) -> Option<String> {
        XyInProcessDriver::model_provider(&self.inner, model_id)
    }

    async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        XyInProcessDriver::set_thinking_level(&mut self.inner, level).await
    }

    async fn ensure_tool_table_frozen(&mut self) {
        XyInProcessDriver::ensure_tool_table_frozen(&mut self.inner).await;
    }

    async fn execute_command(&mut self, cmd: Command) -> Result<serde_json::Value, XyDriverError> {
        let outcome = SessionCommandExecutor::execute_session_command(&mut self.inner, cmd).await?;
        Ok(outcome_to_value(outcome))
    }

    fn agent_cwd_for_test(&self) -> String {
        XyInProcessDriver::agent_cwd_for_test(&self.inner)
    }
}

pub fn host_state_for_test() -> Result<Arc<HostState>, Box<dyn std::error::Error>> {
    host_state_for_test_with(None, None)
}

pub fn host_state_for_test_with_mcp(
    mcp_servers: Vec<McpServerSpec>,
) -> Result<Arc<HostState>, Box<dyn std::error::Error>> {
    host_state_for_test_with(Some(mcp_servers), None)
}

pub fn host_state_for_test_with_default_model(
    model_id: &str,
) -> Result<Arc<HostState>, Box<dyn std::error::Error>> {
    host_state_for_test_with(None, Some(model_id))
}

fn host_state_for_test_with(
    mcp_servers: Option<Vec<McpServerSpec>>,
    default_model_id: Option<&str>,
) -> Result<Arc<HostState>, Box<dyn std::error::Error>> {
    use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind, XyModelMeta};
    use xylitol_runtime::agent::model::registry::ModelRegistry;

    let dir = std::env::temp_dir().join(format!("xylitol-host-{}", uuid::Uuid::new_v4()));
    let sessions = dir.join("sessions");
    let store: Arc<dyn XySessionStore> = Arc::new(
        xylitol_official_plugin_session::SessionManager::new(sessions.clone()),
    );
    let mut options = crate::composition::BuildAgentOptions::default();
    if let Some(model_id) = default_model_id {
        let mut registry = ModelRegistry::new();
        registry.register(XyModelMeta {
            id: model_id.to_string(),
            config: XyModelConfig {
                kind: XyModelKind::Fake,
                api_key: String::new(),
                model: model_id.to_string(),
                base_url: None,
                api: None,
                compat: None,
            },
            display_name: model_id.to_string(),
            thinking: false,
            context_window: 64_000,
            api: String::new(),
            provider: "fake".into(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: vec!["off".into()],
            thinking_level_map: Default::default(),
        });
        options.model_registry = registry;
    }
    let ports = build_baseline_with_store(options, store)?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let agent_dir = xylitol_runtime::resource::DefaultResourceLoader::default_agent_dir();
    let mcp = mcp_servers.unwrap_or_default();
    let default_model = default_model_id.map(str::to_string);
    let factory = Arc::new(ProductOccupancyFactory::new(
        ports.clone(),
        cwd.clone(),
        agent_dir.clone(),
        true,
        mcp,
        default_model.clone(),
    ));
    Ok(HostState::new(
        ports,
        ReloadBaseline {
            cwd,
            agent_dir,
            project_trusted: true,
            default_model_id: default_model,
        },
        factory,
        "test-session".into(),
    )
    .remember_sessions_dir(sessions)
    .arm_test_linger())
}
