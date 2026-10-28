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
pub mod plan;
pub mod process;
pub mod read;
pub mod todo;
pub mod truncate;
pub mod typed;
pub mod write;

pub use ask::default_tools_with_ask;
pub use ask::default_tools_with_ask_and_todo;
pub use plan::{SessionPlanGateway, plan_tools};
pub use todo::{SessionAgentTodoGateway, todo_tools};

use std::sync::Arc;

use xylitol_plugin_sdk::{AgentTodoGateway, XyTool};

/// per-tool `#[cfg(test)]` modules.
#[cfg(test)]
pub(crate) fn test_ctx() -> xylitol_plugin_sdk::XyToolCtx {
    xylitol_plugin_sdk::XyToolCtx::new("test-call")
}

struct MemoryTodoGateway {
    list: tokio::sync::RwLock<TodoListMem>,
}

#[derive(Clone, Default)]
struct TodoListMem(xylitol_plugin_sdk::session::TodoList);

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
    ) -> Result<xylitol_plugin_sdk::session::TodoList, xylitol_plugin_sdk::error::XyToolError> {
        Ok(self.list.read().await.0.clone())
    }

    async fn rewrite(
        &self,
        items: Vec<xylitol_plugin_sdk::session::TodoItemDraft>,
    ) -> Result<xylitol_plugin_sdk::session::TodoList, xylitol_plugin_sdk::error::XyToolError> {
        let list = xylitol_plugin_sdk::session::normalize_rewrite_items(items)
            .map_err(|e| xylitol_plugin_sdk::error::XyToolError::InvalidArgs(e.0))?;
        self.list.write().await.0 = list.clone();
        Ok(list)
    }

    async fn update(
        &self,
        patches: Vec<xylitol_plugin_sdk::session::TodoItemPatch>,
    ) -> Result<xylitol_plugin_sdk::session::TodoList, xylitol_plugin_sdk::error::XyToolError> {
        let current = self.list.read().await.0.clone();
        let list = xylitol_plugin_sdk::session::apply_todo_patches(&current, &patches)
            .map_err(|e| xylitol_plugin_sdk::error::XyToolError::InvalidArgs(e.0))?;
        self.list.write().await.0 = list.clone();
        Ok(list)
    }
}

pub fn default_tools() -> Vec<Arc<dyn XyTool>> {
    default_tools_with_todo(MemoryTodoGateway::new())
}

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

pub fn default_tools_with_todo_plan(
    todo: Arc<dyn AgentTodoGateway>,
    plan: Arc<dyn xylitol_plugin_sdk::PlanGateway>,
) -> Vec<Arc<dyn XyTool>> {
    let mut tools = default_tools_with_todo(todo.clone());
    tools.extend(plan_tools(plan, todo, None));
    tools
}

pub fn default_tools_with_ask_todo_plan(
    ask: Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>,
    todo: Arc<dyn AgentTodoGateway>,
    plan: Arc<dyn xylitol_plugin_sdk::PlanGateway>,
) -> Vec<Arc<dyn XyTool>> {
    let mut tools = default_tools_with_ask_and_todo(ask.clone(), todo.clone());
    tools.extend(plan_tools(plan, todo, Some(ask)));
    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::XyToolExecutionMode;

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
        for name in ["write", "edit", "bash", "todo_rewrite", "todo_update"] {
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
        assert_eq!(names.len(), 9);
        for n in [
            "read",
            "bash",
            "edit",
            "write",
            "grep",
            "find",
            "ls",
            "todo_rewrite",
            "todo_update",
        ] {
            assert!(names.contains(&n), "missing {n}");
        }
        assert!(!names.contains(&"ask"));
    }
}
