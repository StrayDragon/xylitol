mod in_process;

pub(crate) use in_process::attach_ctx_services;
pub(crate) use in_process::install_product_ask_tool;
pub use in_process::{
    DriverContext, ToolFeatures, XyInProcessDriver, assemble_driver, assemble_session_driver,
};
pub use xylitol_runtime::driver::{
    CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot, MCP_PENDING_CUE,
    McpServerPhase, McpServerSnapshot, ModelInfo, ProjectTrustMode, ProjectTrustPersistReport,
    QueueStats, ReloadStepReport, RuntimeReloadReport, SessionListEntry, SessionState,
    SessionStats, TurnRequest, XyEvent, estimate_from_session_entries,
};
pub use xylitol_runtime::{
    BangExecHandler, BashRun, SessionExporter, XyDriver, XyDriverError, XySessionOps,
};

mod command_exec;
pub use command_exec::CommandExecutorView;
