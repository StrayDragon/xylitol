use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::{Mutex, MutexGuard};

use xylitol_plugin_sdk::ask::AskUserGateway;
use xylitol_protocol::Command;
use xylitol_runtime::driver::ModelInfo;
use xylitol_runtime::{XyDriver, XyDriverError};

#[async_trait]
pub trait OccupancyEngine: XyDriver {
    async fn join_actor(&mut self);
    fn install_ask_tool(&mut self, gateway: Arc<dyn AskUserGateway>);
    fn bind_session(&mut self, session_id: &str) -> Result<(), XyDriverError>;
    async fn restore_model(&mut self, model_id: &str) -> Result<(), XyDriverError>;
    async fn switch_session(&mut self, session_id: &str) -> Result<String, XyDriverError>;
    async fn select_model(&mut self, model_id: &str) -> Result<ModelInfo, XyDriverError>;
    fn model_provider(&self, model_id: &str) -> Option<String>;
    async fn set_thinking_level(&mut self, level: String) -> Result<(), XyDriverError>;
    async fn ensure_tool_table_frozen(&mut self);
    async fn poll_tool_gate(&mut self) -> bool;
    async fn execute_command(&mut self, cmd: Command) -> Result<Value, XyDriverError>;
    /// ~keep 测试接缝：trait 层暴露 agent 工作目录，仅供集成测试观测，非产品 API。
    /// ~keep 调用方：`xylitol-kernel` occupancy factory 经此转发，最终由 `tests/occupancy/assembly_fixture_tests.rs` 断言。
    /// ~keep 不得挂 `#[cfg(test)]`：调用方跨 crate，cfg 只在被测 crate 自身测试编译时激活。
    fn agent_cwd_for_test(&self) -> String;
    fn set_loop_middleware_scoped(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
        scope: xylitol_plugin_sdk::LoopScope,
    );

    fn set_workspaces(&mut self, _workspaces: Vec<String>) {}
}

#[derive(Clone)]
pub struct SessionDriver {
    cell: Arc<Mutex<Option<Box<dyn OccupancyEngine>>>>,
}

impl SessionDriver {
    pub fn vacant() -> Self {
        Self {
            cell: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn lock(&self) -> MutexGuard<'_, Option<Box<dyn OccupancyEngine>>> {
        self.cell.lock().await
    }

    pub async fn release(&self) -> Option<Box<dyn OccupancyEngine>> {
        self.cell.lock().await.take()
    }

    pub async fn release_and_join(&self) {
        if let Some(mut engine) = self.release().await {
            engine.join_actor().await;
        }
    }
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

    fn release_session(&self, _session_id: &str) {}
}
