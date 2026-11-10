use async_trait::async_trait;

#[async_trait]
pub trait AutomationGateway: Send + Sync {
    async fn bind_session(&self, session_id: Option<String>);
}
