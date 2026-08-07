//! Agent runtime — the thin orchestration loop and its collaborators.
//!
//! Houses the ReAct loop (`react`), its event vocabulary (`event`), hook
//! extension points (`hooks`), transient-error retry, and stdout takeover
//! guard. The `agent` module is the public entry; modules here are the loop's
//! internals.

pub mod event;
pub mod hooks;
pub(crate) mod obs;
pub(crate) mod permission_router;
pub mod ports;
pub mod react;
pub mod retry;
pub(crate) mod script_hook_ctx;
pub mod state;
pub(crate) mod tool_batch;
pub(crate) mod tool_exec;
pub(crate) mod tool_result_quiet;

// Public surface of the loop (XyEvent/XyEventStream/AgentRuntime/AgentHooks are `pub`).
pub use crate::protocol::lifecycle::XyEvent;
pub use event::XyEventStream;
pub use hooks::AgentHooks;
pub use ports::RuntimePorts;
pub use react::AgentRuntime;
pub use state::{RunId, RunPolicy, RuntimeControlError, RuntimePhase};
