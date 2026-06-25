//! Agent runtime — the thin orchestration loop and its collaborators.
//!
//! Houses the ReAct loop (`react`), the steer/follow-up message queue,
//! transient-error retry, and stdout takeover guard. The agent facade is the
//! public entry; modules here are the loop's internals.

pub mod queue;
pub mod react;
pub mod retry;
pub mod stdout_guard;

// Public surface of the loop (AgentEvent/AgentLoop/AgentHooks are `pub`).
pub use react::{AgentEvent, AgentEventStream, AgentHooks, AgentLoop};

// Crate-internal collaborators consumed via the runtime path.
pub(crate) use queue::MessageQueue;
