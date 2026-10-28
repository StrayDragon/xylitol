mod in_process;

pub use in_process::{
    ToolFeatures, XyInProcessDriver, assemble_driver, assemble_driver_with_bang,
    assemble_driver_with_features, assemble_driver_with_ops, assemble_driver_with_ops_and_features,
    install_product_ask_tool,
};
pub use xylitol_runtime::driver::{
    ClipboardCopyOutcome, CommandInfo, DebugSceneLoad, EventStream, LoadedResourcesSnapshot,
    MCP_PENDING_CUE, McpServerPhase, McpServerSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, QueueStats, ReloadStepReport, RuntimeReloadReport, SessionListEntry,
    SessionState, SessionStats, XyEvent, estimate_from_session_entries,
};
pub use xylitol_runtime::{
    AttachObserve, BangExecHandler, BashRun, LinkHealth, SessionExporter, XyDriver, XyDriverError,
};

mod command_exec;
