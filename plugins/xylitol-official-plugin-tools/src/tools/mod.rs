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

pub use ask::{ask_tool, default_tools_with_ask};

use std::sync::Arc;

use xylitol_plugin_sdk::XyTool;

/// per-tool `#[cfg(test)]` modules.
#[cfg(test)]
pub(crate) fn test_ctx() -> xylitol_plugin_sdk::XyToolCtx {
    xylitol_plugin_sdk::XyToolCtx::new("test-call")
}

pub fn core_tools() -> Vec<Arc<dyn XyTool>> {
    let mq = Arc::new(mutation::FileMutationQueue::new());
    vec![
        Arc::new(read::ReadTool),
        Arc::new(write::WriteTool::new(mq.clone())),
        Arc::new(edit::EditTool::new(mq)),
        Arc::new(bash::BashTool::default()),
        Arc::new(grep::GrepTool),
        Arc::new(find::FindTool),
        Arc::new(ls::LsTool),
    ]
}

pub fn default_tools() -> Vec<Arc<dyn XyTool>> {
    core_tools()
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
        for name in ["write", "edit", "bash"] {
            assert_eq!(
                mode(name),
                XyToolExecutionMode::Sequential,
                "{name} must be Barrier"
            );
        }
    }

    #[test]
    fn default_tools_are_core_only() {
        let tools = default_tools();
        let names: Vec<_> = tools.iter().map(|t| t.name()).collect();
        assert_eq!(names.len(), 7);
        for n in ["read", "bash", "edit", "write", "grep", "find", "ls"] {
            assert!(names.contains(&n), "missing {n}");
        }
        assert!(!names.contains(&"ask"));
        assert!(!names.contains(&"todo_rewrite"));
        assert!(!names.contains(&"plan_write"));
    }
}
