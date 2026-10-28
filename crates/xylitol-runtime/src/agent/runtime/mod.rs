pub mod baseline;
pub mod event;
pub mod hooks;
pub(crate) mod middleware;
pub(crate) mod obs;
pub(crate) mod permission_router;
pub mod react;
pub mod retry;
pub mod script_hook_ctx;
pub mod state;
pub(crate) mod tool_batch;
pub(crate) mod tool_exec;
pub(crate) mod tool_result_quiet;

pub use baseline::RuntimeBaseline;
pub use event::XyEventStream;
pub use hooks::AgentHooks;
pub use react::AgentRuntime;
pub use state::{
    RunCoordinator, RunId, RunPolicy, RuntimeControlError, RuntimePhase, SharedRunCoordinator,
};
pub use xylitol_plugin_sdk::lifecycle::XyEvent;
