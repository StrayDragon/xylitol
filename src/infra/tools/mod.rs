//! Built-in tool implementations and the default registry factory.
//!
//! Each tool implements [`crate::runtime_protocol::XyTool`]. The composition root
//! (and tests) build a registry via [`default_registry`]; the agent holds the
//! resulting registry as orchestration state without naming these types.

pub mod accumulator;
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
pub mod write;

use std::sync::Arc;

use crate::runtime_protocol::XyTool;

/// Return all built-in tools as trait objects.
///
/// The composition root (`app::cli`) passes this list to
/// `ToolRegistry::new()` + `register()`. The agent never constructs concrete
/// tool types; infra never names the agent's `ToolRegistry`.
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
