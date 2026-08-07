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
pub mod truncate;
pub mod typed;
pub mod write;

pub use ask::{AskTool, default_tools_with_ask};

use std::sync::Arc;

use crate::protocol::ports::XyTool;

/// Return all built-in tools as trait objects.
///
/// The composition root (`app::cli`) passes this list to
/// `ToolSet::new()` + `register()`. The agent never constructs concrete
/// tool types; infra never names the agent's `ToolSet`.
pub fn default_tools() -> Vec<Arc<dyn XyTool>> {
    let mq = Arc::new(mutation::FileMutationQueue::new());
    vec![
        Arc::new(read::ReadTool),
        Arc::new(write::WriteTool::new(mq.clone())),
        Arc::new(edit::EditTool::new(mq.clone())),
        Arc::new(bash::BashTool::default()),
        Arc::new(grep::GrepTool),
        Arc::new(find::FindTool),
        Arc::new(ls::LsTool),
    ]
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
        for name in ["write", "edit", "bash"] {
            assert_eq!(
                mode(name),
                XyToolExecutionMode::Sequential,
                "{name} must be Barrier"
            );
        }
    }
}
