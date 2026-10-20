//! XyDriver abstraction — the single dependency of interactive layers.
//!
//! Runtime core lives in `xylitol-runtime`. This module adds the composition-root
//! wrapper (official plugins + MCP reload). Remote attach lives in `xylitol-app-tui`.

mod clipboard;
mod in_process;

pub use in_process::XyInProcessDriver;
pub use xylitol_runtime::driver::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    MCP_PENDING_CUE, McpServerPhase, McpServerSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, QueueStats, ReloadStepReport, RuntimeReloadReport, SessionListEntry,
    SessionState, SessionStats, XyEvent, estimate_from_session_entries,
};
pub use xylitol_runtime::{
    AttachObserve, BangExecHandler, BashRun, LinkHealth, SessionExporter, XyDriver, XyDriverError,
};

/// Envelope `Command` dispatch lives beside the product driver (runtime has no envelope).
mod command_exec;
