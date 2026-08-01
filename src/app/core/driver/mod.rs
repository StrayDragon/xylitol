//! XyDriver abstraction — the single dependency of interactive layers.
//!
//! All interactive clients (cli/print/tui) interact with the core through a
//! [`XyDriver`]; they import agent symbols only from `agent` (mod-level),
//! never reaching into `agent::session`/`agent::runtime` internals or `infra`.
//!
//! - [`XyInProcessDriver`]: wraps the local agent module (composition root wires
//!   ports and agent together).
//! - [`XyRemoteDriver`]: speaks the protocol over REST/WS to a remote server.
//!
//! The trait carries not just `run`/`abort` but the full set of command
//! execution semantics (model selection, compaction, export, session ops) so
//! that [`crate::app::core::dispatch`] can be a pure Command→method dispatcher
//! shared by tui (spec ce10). WS-transport-specific commands
//! (Subscribe/ApproveTool/AnswerQuestion) do NOT live here — they stay in
//! `app::server::ws`.
//!
//! NOTE: trait methods are consumed via dispatch under tui/server features.
//! `XyRemoteDriver` is reserved for a remote thin-client surface (not constructed
//! yet) — `dead_code` allow is on that type/impl, not this module.

mod in_process;
mod proto;
#[cfg(feature = "server")]
mod remote;
mod types;

pub use crate::app::core::driver_error::XyDriverError;
pub use in_process::XyInProcessDriver;
pub use proto::XyDriver;
#[cfg(feature = "server")]
#[allow(unused_imports)] // reserved remote thin-client surface
pub use remote::XyRemoteDriver;
// Re-export seam DTOs for surfaces (`crate::app::core::driver::*`).
#[allow(unused_imports)]
pub use types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    MCP_PENDING_CUE, McpServerPhase, McpServerSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, QueueStats, ReloadStepReport, RuntimeReloadReport, SessionListEntry,
    SessionState, SessionStats, XyEvent, estimate_from_session_entries,
    tokenizer_override_from_app_config,
};
