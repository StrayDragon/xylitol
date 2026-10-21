pub use std::cell::{Cell, RefCell};
pub use std::collections::HashMap;
pub use std::sync::Arc;

pub use futures::StreamExt;
pub use xylitol::XyDriverError;
pub use xylitol_kernel::config::types::HookEntry;
pub use xylitol_kernel::provider::factory::{
    reset_fake_state, set_fake_slow_stream, set_fake_text, set_fake_tool_call, set_fake_tool_result,
};
pub use xylitol_official_plugin_compaction::should_compact;
pub use xylitol_official_plugin_hooks::{DispatchResult, HookEvent, HookPhase};
pub use xylitol_official_plugin_session::{
    BranchSummaryEntry, CompactionEntry, EntryBase, MessageEntry, SessionEntry, SessionManager,
    ThinkingLevelChangeEntry,
};
pub use xylitol_official_plugin_tools::{
    bash::BashTool, edit::EditTool, find::FindTool, grep::GrepTool, ls::LsTool,
    mutation::FileMutationQueue, read::ReadTool, write::WriteTool,
};
pub use xylitol_plugin_sdk::model::XyModelMeta;
pub use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind};
pub use xylitol_plugin_sdk::session::ForkPosition;
pub use xylitol_plugin_sdk::{XyTool, XyToolCtx};
pub(crate) use xylitol_runtime::agent::capabilities::{
    AgentCapabilities, ModelRegistry, get_context_usage,
};
pub use xylitol_runtime::agent::runtime::{AgentRuntime, XyEvent};
pub use xylitol_runtime::agent::tools::ToolSet;
