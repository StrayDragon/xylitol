pub mod builder;
pub mod capabilities;
pub mod compaction;
pub mod context_policy;
pub mod llm_project;
pub mod model;
pub mod prompt;
pub mod runtime;
pub mod tools;

pub use crate::agent::builder::AgentBuilder;
pub use crate::agent::capabilities::{
    HookBlockedError, ModelRegistry, QueueMode, QueueStats, SessionStats,
};
pub use crate::agent::runtime::AgentRuntime;
pub use crate::agent::runtime::RuntimeBaseline;
pub use crate::agent::runtime::XyEventStream;
pub use crate::agent::runtime::hooks::max_turns_stop_hook;
pub use crate::agent::tools::MCP_FIRST_TURN_GATE_TIMEOUT;
pub use xylitol_plugin_sdk::lifecycle::XyEvent;
