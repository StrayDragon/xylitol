//! Built-in tool implementations and the default registry factory.
//!
//! Each tool implements [`crate::protocol::ports::XyTool`] (directly or via
//! [`typed::TypedTool`] blanket). The composition root (and tests) build a
//! registry via [`default_tools`]; the agent holds the resulting registry as
//! orchestration state without naming these types.

pub mod accumulator;
pub mod args;
pub mod ask;
pub mod bash;
pub mod edit;
pub mod find;
pub mod grep;
pub mod ls;
pub mod mutation;
pub mod patch;
pub mod path_utils;
pub mod process;
pub mod read;
pub mod todo;
pub mod truncate;
pub mod typed;
pub mod write;

pub use ask::{AskTool, default_tools_with_ask, default_tools_with_ask_and_todo};
pub use todo::{
    SessionAgentTodoGateway, TodoListTool, TodoRewriteTool, TodoUpdateTool, todo_tools,
};

use std::sync::Arc;

use crate::protocol::ports::{AgentTodoGateway, XyTool};

/// Ephemeral in-memory Todo gateway for call sites that do not bind a session
/// (unit / BDD name tables). Product surfaces SHOULD prefer
/// [`default_tools_with_todo`] with a [`SessionAgentTodoGateway`].
struct MemoryTodoGateway {
    list: tokio::sync::RwLock<TodoListMem>,
}

#[derive(Clone, Default)]
struct TodoListMem(crate::protocol::session::TodoList);

impl MemoryTodoGateway {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            list: tokio::sync::RwLock::new(TodoListMem::default()),
        })
    }
}

#[async_trait::async_trait]
impl AgentTodoGateway for MemoryTodoGateway {
    async fn list(
        &self,
    ) -> Result<crate::protocol::session::TodoList, crate::protocol::error::XyToolError> {
        Ok(self.list.read().await.0.clone())
    }

    async fn rewrite(
        &self,
        items: Vec<crate::protocol::session::TodoItemDraft>,
    ) -> Result<crate::protocol::session::TodoList, crate::protocol::error::XyToolError> {
        let list = crate::protocol::session::normalize_rewrite_items(items)
            .map_err(|e| crate::protocol::error::XyToolError::InvalidArgs(e.0))?;
        self.list.write().await.0 = list.clone();
        Ok(list)
    }

    async fn update(
        &self,
        id: &str,
        status: Option<crate::protocol::session::TodoStatus>,
        content: Option<String>,
    ) -> Result<crate::protocol::session::TodoList, crate::protocol::error::XyToolError> {
        let current = self.list.read().await.0.clone();
        let list = crate::protocol::session::apply_todo_update(&current, id, status, content)
            .map_err(|e| crate::protocol::error::XyToolError::InvalidArgs(e.0))?;
        self.list.write().await.0 = list.clone();
        Ok(list)
    }
}

/// Return all built-in tools as trait objects (10-tool closed set).
///
/// Uses an ephemeral Todo gateway (no session persistence). Product composition
/// SHOULD call [`default_tools_with_todo`] with a store-bound gateway.
pub fn default_tools() -> Vec<Arc<dyn XyTool>> {
    default_tools_with_todo(MemoryTodoGateway::new())
}

/// Builtins with an injected Todo gateway (Print + TUI base).
pub fn default_tools_with_todo(todo: Arc<dyn AgentTodoGateway>) -> Vec<Arc<dyn XyTool>> {
    let mq = Arc::new(mutation::FileMutationQueue::new());
    let mut tools: Vec<Arc<dyn XyTool>> = vec![
        Arc::new(read::ReadTool),
        Arc::new(write::WriteTool::new(mq.clone())),
        Arc::new(edit::EditTool::new(mq.clone())),
        Arc::new(bash::BashTool::default()),
        Arc::new(grep::GrepTool),
        Arc::new(find::FindTool),
        Arc::new(ls::LsTool),
    ];
    tools.extend(todo_tools(todo));
    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ports::XyToolExecutionMode;

    #[test]
    fn builtin_concurrency_class_table() {
        let tools = default_tools();
        let mode = |name: &str| {
            tools
                .iter()
                .find(|t| t.name() == name)
                .unwrap_or_else(|| panic!("missing tool {name}"))
                .execution_mode()
        };
        for name in ["read", "grep", "find", "ls"] {
            assert_eq!(
                mode(name),
                XyToolExecutionMode::Parallel,
                "{name} must be ParallelSafe"
            );
        }
        for name in [
            "write",
            "edit",
            "bash",
            "todo_list",
            "todo_rewrite",
            "todo_update",
        ] {
            assert_eq!(
                mode(name),
                XyToolExecutionMode::Sequential,
                "{name} must be Barrier"
            );
        }
    }

    #[test]
    fn default_tools_include_todo_closed_set() {
        let tools = default_tools();
        let names: Vec<_> = tools.iter().map(|t| t.name()).collect();
        assert_eq!(names.len(), 10);
        for n in [
            "read",
            "bash",
            "edit",
            "write",
            "grep",
            "find",
            "ls",
            "todo_list",
            "todo_rewrite",
            "todo_update",
        ] {
            assert!(names.contains(&n), "missing {n}");
        }
        assert!(!names.contains(&"ask"));
    }
}
