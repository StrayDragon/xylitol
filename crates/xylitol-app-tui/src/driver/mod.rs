//! Client-side XyDriver seam (envelope adapter + scripted tests).

mod clipboard;
pub use clipboard::{copy_text_to_clipboard, read_clipboard_text, stage_clipboard_image};
mod proto;
mod remote;
mod types;

pub use crate::driver_error::XyDriverError;
pub use proto::BashRun;
pub use proto::{AttachObserve, LinkHealth, XyDriver};
pub use remote::{LinkTunings, XyRemoteDriver};
#[allow(unused_imports)]
pub use types::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    MCP_PENDING_CUE, McpServerPhase, McpServerSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, QueueStats, ReloadStepReport, RuntimeReloadReport, SessionListEntry,
    SessionState, SessionStats, XyEvent, estimate_from_session_entries,
};
