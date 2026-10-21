//! In-process [`XyDriver`]: ReAct + session store. No envelope, no official plugins.

mod bang_exec;
mod error;
mod in_process;
mod proto;
mod session_export;
mod types;

pub use bang_exec::BangExecHandler;
pub use error::XyDriverError;
pub use in_process::{InProcessReloadHook, XyInProcessDriver, relay_bash_chunks, utf8_emit_len};
pub use proto::{AttachObserve, BashRun, LinkHealth, XyDriver};
pub use session_export::SessionExporter;
pub use types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    MCP_PENDING_CUE, McpServerPhase, McpServerSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, QueueStats, ReloadStepReport, RuntimeReloadReport, SessionListEntry,
    SessionState, SessionStats, XyEvent, estimate_from_session_entries,
};
