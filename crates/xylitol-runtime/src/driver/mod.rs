mod bang_exec;
mod error;
mod in_process;
mod proto;
mod session_export;
mod types;

pub use bang_exec::BangExecHandler;
pub use error::XyDriverError;
pub use in_process::{
    InProcessReloadHook, ReloadContext, ReloadParticipant, ReloadRegistry, XyInProcessDriver,
    XySessionOps, relay_bash_chunks, utf8_emit_len,
};
pub use proto::{BashRun, XyDriver};
pub use session_export::SessionExporter;
pub use types::{
    CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot, MCP_PENDING_CUE,
    McpServerPhase, McpServerSnapshot, ModelInfo, ProjectTrustMode, ProjectTrustPersistReport,
    QueueStats, ReloadStepReport, RuntimeReloadReport, SessionListEntry, SessionState,
    SessionStats, TurnRequest, XyEvent, estimate_from_session_entries,
};
