use crate::error::XyToolError;
use crate::session::{TodoItemDraft, TodoItemPatch, TodoList};

#[async_trait::async_trait]
pub trait AgentTodoGateway: Send + Sync {
    async fn list(&self) -> Result<TodoList, XyToolError>;

    async fn rewrite(&self, items: Vec<TodoItemDraft>) -> Result<TodoList, XyToolError>;

    async fn update(&self, patches: Vec<TodoItemPatch>) -> Result<TodoList, XyToolError>;

    async fn bind_session(&self, session_id: Option<String>) {
        let _ = session_id;
    }
}
