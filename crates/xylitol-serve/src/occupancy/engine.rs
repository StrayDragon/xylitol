use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use xylitol_plugin_sdk::ask::AskUserGateway;
use xylitol_protocol::Command;
use xylitol_runtime::driver::ModelInfo;
use xylitol_runtime::{XyDriver, XyDriverError};

#[async_trait]
pub trait OccupancyEngine: XyDriver {
    fn install_ask_tool(&mut self, gateway: Arc<dyn AskUserGateway>);
    fn bind_session(&mut self, session_id: &str) -> Result<(), XyDriverError>;
    async fn restore_model(&mut self, model_id: &str) -> Result<(), XyDriverError>;
    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError>;
    async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError>;
    fn model_provider(&self, model_id: &str) -> Option<String>;
    async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError>;
    async fn ensure_tool_table_frozen(&mut self);
    async fn execute_command(&mut self, cmd: Command) -> Result<Value, XyDriverError>;
    fn agent_cwd_for_test(&self) -> String;
    fn set_loop_middleware_scoped(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
        scope: xylitol_plugin_sdk::LoopScope,
    );
}

pub trait OccupancyFactory: Send + Sync {
    fn new_driver(&self) -> Box<dyn OccupancyEngine> {
        self.new_driver_at(&self.cwd())
    }
    fn new_driver_at(&self, workspace: &std::path::Path) -> Box<dyn OccupancyEngine>;
    fn project_trusted(&self, workspace: &std::path::Path) -> bool;
    fn default_model_id(&self) -> Option<String>;
    fn cwd(&self) -> PathBuf;
    fn agent_dir(&self) -> PathBuf;

    fn loop_registry(&self) -> Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>> {
        None
    }
}
