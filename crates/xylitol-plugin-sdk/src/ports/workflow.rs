use async_trait::async_trait;

use crate::error::XyToolError;
use crate::session::Workflow;

#[async_trait]
pub trait WorkflowGateway: Send + Sync {
    async fn latest(&self) -> Result<Option<Workflow>, XyToolError>;

    async fn persist(&self, workflow: &Workflow) -> Result<(), XyToolError>;

    async fn bind_session(&self, _session_id: Option<String>) {}
}
