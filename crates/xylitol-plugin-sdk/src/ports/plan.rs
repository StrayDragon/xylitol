use async_trait::async_trait;

use crate::error::XyToolError;
use crate::session::{PlanDoc, PlanMode};

#[async_trait]
pub trait PlanGateway: Send + Sync {
    async fn list(&self) -> Result<Vec<PlanDoc>, XyToolError>;

    async fn read(&self, plan_id: Option<&str>) -> Result<Option<PlanDoc>, XyToolError>;

    async fn write(&self, doc: PlanDoc) -> Result<PlanDoc, XyToolError>;

    async fn mode(&self) -> Result<PlanMode, XyToolError>;

    async fn set_mode(&self, mode: PlanMode) -> Result<(), XyToolError>;

    async fn bind_session(&self, _session_id: Option<String>) {}
}
