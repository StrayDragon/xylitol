//! Ports — boundary traits between the `agent` and `infra` layers.
//!
//! Trait contracts and signature-only associated types.

pub mod ask;
pub mod bash;
pub mod compaction;
pub mod event;
pub mod export;
pub mod hook;
pub mod http;
pub mod model;
pub mod permission;
pub mod secret;
pub mod session;
pub mod session_list;
pub mod todo;
pub mod tool;
pub mod typed;

// Flat re-exports for the common case where callers import the boundary
// trait directly from `protocol::ports` (or via `protocol` root) rather than
// from its submodule.
pub use ask::{AskArgs, AskModeArg, AskOptionArg, AskQuestionArg, AskUserGateway};
pub use bash::{BashChunk, BashExecOpts, BashOutputSink, XyBashExecutor, XyBashResult};
pub use compaction::{
    NoopCompaction, OverflowCompactOutcome, XyCompaction, XyCompactionFactory,
    noop_compaction_factory, should_compact,
};
pub use event::{LifecycleHandler, XyEventSink};
pub use export::XyExportIo;
pub use hook::{NoopHookBus, XyHookBus, XyHookOutcome};
pub use http::{HeaderBag, HttpHooks, run_after_response, run_before_headers, run_before_request};
pub use model::{XyGenerateOptions, XyModel, XyModelBuilder, XyStream};
pub use permission::{XyPermission, XyPermissionVerdict};
pub use secret::XySecretResolver;
pub use session::{SessionListEntry, XySessionStore, sanitize_session_display_name};
pub use session_list::{flatten_session_forest, format_session_age};
pub use todo::AgentTodoGateway;
pub use tool::{XyBatchMode, XyTool, XyToolCtx, XyToolExecutionMode};
pub use typed::{FS_TOOL_TIMEOUT_SECS, TypedTool, parse_tool_args};
