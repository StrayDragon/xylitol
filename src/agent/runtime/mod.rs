//! Agent runtime — the thin orchestration loop and its collaborators.
//!
//! Houses the ReAct loop (`react`), its event vocabulary (`event`), hook
//! extension points (`hooks`), the steer/follow-up message queue,
//! transient-error retry, and stdout takeover guard. The agent facade is the
//! public entry; modules here are the loop's internals.

pub mod event;
pub mod hooks;
pub mod queue;
pub mod react;
pub mod retry;
pub(crate) mod sandbox_router;

// Public surface of the loop (AgentEvent/AgentLoop/AgentHooks are `pub`).
pub use event::{AgentEvent, AgentEventStream};
pub use hooks::AgentHooks;
pub use react::AgentLoop;

// Crate-internal collaborators consumed via the runtime path.
pub(crate) use queue::MessageQueue;
