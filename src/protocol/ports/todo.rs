//! Agent Todo gateway — builtin `todo_*` tools ↔ session SSOT.
//!
//! Tools in `infra` call this port; composition / tests bind a store-backed or
//! in-memory implementation. Keeps session mutation out of `XyToolCtx`.

use crate::protocol::error::XyToolError;
use crate::protocol::session::{TodoItemDraft, TodoList, TodoStatus};

/// Session-bound Todo SSOT operations for builtin tools.
#[async_trait::async_trait]
pub trait AgentTodoGateway: Send + Sync {
    /// Current full list (empty when none).
    async fn list(&self) -> Result<TodoList, XyToolError>;

    /// Replace entire list; persist snapshot on success.
    async fn rewrite(&self, items: Vec<TodoItemDraft>) -> Result<TodoList, XyToolError>;

    /// Patch one item by id; persist snapshot on success.
    async fn update(
        &self,
        id: &str,
        status: Option<TodoStatus>,
        content: Option<String>,
    ) -> Result<TodoList, XyToolError>;
}
