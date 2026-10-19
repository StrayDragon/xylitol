//! Agent layer — re-exports `xylitol-runtime`. Compaction implementation stays
//! the official plugin (injected at the composition root).

pub use xylitol_runtime::agent::{
    AgentBuilder, AgentRuntime, HookBlockedError, MCP_FIRST_TURN_GATE_TIMEOUT, ModelRegistry,
    QueueMode, QueueStats, RuntimePorts, SessionStats, XyEvent, XyEventStream, max_turns_stop_hook,
};

pub mod builder {
    pub use xylitol_runtime::agent::builder::*;
}
pub mod capabilities {
    pub use xylitol_runtime::agent::capabilities::*;
}
pub mod compaction {
    pub use xylitol_official_plugin_compaction::*;
    pub use xylitol_official_plugin_compaction::{
        binding, cut_detector, file_ops, llm_summarizer, obs, orchestrator, overflow, settings,
        settlement, token_estimator,
    };
}
pub mod context_policy {
    pub use xylitol_runtime::agent::context_policy::*;
}
pub mod llm_project {
    pub use xylitol_runtime::agent::llm_project::*;
}
pub mod model {
    pub use xylitol_runtime::agent::model::*;
}
pub mod prompt {
    pub use xylitol_runtime::agent::prompt::*;
}
pub mod runtime {
    pub use xylitol_runtime::agent::runtime::*;
}
pub mod tools {
    pub use xylitol_runtime::agent::tools::*;
}
