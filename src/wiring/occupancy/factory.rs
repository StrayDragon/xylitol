//! Composition-root occupancy factory: injects official plugins into serve slots.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::ask::AskUserGateway;
use xylitol_protocol::Command;
use xylitol_runtime::XyDriverError;
use xylitol_runtime::agent::RuntimeBaseline;
use xylitol_serve::occupancy::{OccupancyEngine, OccupancyFactory};

use crate::wiring::composition::build_baseline_with_store;
use crate::wiring::dispatch::{SessionCommandExecutor, outcome_to_value};
use crate::wiring::driver::XyInProcessDriver;
use crate::wiring::mcp_spec::McpServerSpec;
use crate::wiring::occupancy::host::{HostState, ReloadBaseline};

pub struct ProductOccupancyFactory {
    ports: RuntimeBaseline,
    cwd: PathBuf,
    agent_dir: PathBuf,
    project_trusted: bool,
    mcp_servers: Vec<McpServerSpec>,
    default_model_id: Option<String>,
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
        }
    }
}

impl OccupancyFactory for ProductOccupancyFactory {
    fn new_driver_at(&self, workspace: &Path) -> Box<dyn OccupancyEngine> {
        let agent = self.ports.materialize_runtime();
        let mut driver = XyInProcessDriver::new(agent, self.ports.store.clone());
        driver.enable_reload_state(
            workspace.to_path_buf(),
            self.agent_dir.clone(),
            self.project_trusted(workspace),
            self.mcp_servers.clone(),
        );
        driver
            .inner
            .agent
            .set_cwd(workspace.to_string_lossy().into_owned());
        Box::new(driver)
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

#[async_trait]
impl OccupancyEngine for XyInProcessDriver {
    fn install_ask_tool(&mut self, gateway: Arc<dyn AskUserGateway>) {
        XyInProcessDriver::install_ask_tool(self, gateway);
    }

    fn bind_session(&mut self, session_id: &str) -> Result<(), XyDriverError> {
        self.inner
            .agent
            .bind_session(session_id.to_string())
            .map_err(XyDriverError::from)
    }

    async fn restore_model(&mut self, model_id: &str) -> Result<(), XyDriverError> {
        self.inner.restore_model(model_id).await
    }

    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError> {
        self.inner.switch_session(session_id).await
    }

    async fn select_model(
        &mut self,
        model_id: &str,
    ) -> Result<xylitol_runtime::driver::ModelInfo, XyDriverError> {
        self.inner.select_model(model_id).await
    }

    fn model_provider(&self, model_id: &str) -> Option<String> {
        self.inner.model_provider(model_id)
    }

    async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError> {
        self.inner.set_thinking_level(level).await
    }

    async fn ensure_tool_table_frozen(&mut self) {
        XyInProcessDriver::ensure_tool_table_frozen(self).await;
    }

    async fn execute_command(&mut self, cmd: Command) -> Result<serde_json::Value, XyDriverError> {
        let outcome = SessionCommandExecutor::execute_session_command(self, cmd).await?;
        Ok(outcome_to_value(outcome))
    }

    fn agent_cwd_for_test(&self) -> String {
        self.inner.agent_cwd_for_test()
    }
}

/// Isolated store + default ports (BDD / unit tests; no user `~/.xylitol`).
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
    let mut options = crate::wiring::composition::BuildAgentOptions::default();
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
