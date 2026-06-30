//! Agent runtime — the thin orchestration loop and its collaborators.
//!
//! Houses the ReAct loop (`react`), its event vocabulary (`event`), hook
//! extension points (`hooks`), transient-error retry, and stdout takeover
//! guard. The `agent` module is the public entry; modules here are the loop's
//! internals.

pub mod event;
pub mod hooks;
pub(crate) mod permission_router;
pub mod react;
pub mod retry;

// Public surface of the loop (XyEvent/XyEventStream/ReActAgent/AgentHooks are `pub`).
pub use crate::domain::lifecycle::XyEvent;
pub use event::XyEventStream;
pub use hooks::AgentHooks;
pub use react::ReActAgent;
