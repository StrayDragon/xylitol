//! XyDriver abstraction — the single dependency of interactive layers.
//!
//! All interactive clients (cli/print/tui) interact with the core through a
//! [`XyDriver`]; they import agent symbols only from `agent` (mod-level),
//! never reaching into `agent::capabilities`/`agent::runtime` internals or `infra`.
//!
//! - [`XyInProcessDriver`]: wraps the local agent module (composition root wires
//!   ports and agent together).
//! - [`XyRemoteDriver`]: `XyDriver` adapter over [`super::host_client::HttpWsClient`].
//!
//! The trait carries not just `run`/`abort` but the full set of command
//! execution semantics (model selection, compaction, export, session ops) so
//! that [`crate::app::core::dispatch`] can be a pure Command→method dispatcher
//! shared by tui (spec ce10). `subscribe` is a Host unary; reverse-RPC answers
//! are `approve_tool` / `answer_question` unary (not a separate HTTP path).
//!
//! Product TUI attach uses [`HttpWsClient`](super::host_client::HttpWsClient);
//! this adapter keeps the existing [`XyDriver`] seam until TUI speaks the envelope trait directly.

mod clipboard;
mod in_process;
mod proto;

pub use proto::BashRun;
#[cfg(feature = "server")]
mod remote;
mod types;

pub use crate::app::core::driver_error::XyDriverError;
pub use in_process::XyInProcessDriver;
pub use proto::{LinkHealth, XyDriver};
#[cfg(feature = "server")]
pub use remote::{LinkTunings, XyRemoteDriver};
// Re-export seam DTOs for surfaces (`crate::app::core::driver::*`).
#[allow(unused_imports)]
pub use types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    MCP_PENDING_CUE, McpServerPhase, McpServerSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, QueueStats, ReloadStepReport, RuntimeReloadReport, SessionListEntry,
    SessionState, SessionStats, XyEvent, estimate_from_session_entries,
    tokenizer_override_from_app_config,
};
