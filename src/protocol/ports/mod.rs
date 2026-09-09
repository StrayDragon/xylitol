//! Ports — boundary traits between the `agent` and `infra` layers.
//!
//! Trait contracts and signature-only associated types. Depends on protocol-root
//! shared types (and MAY depend on `xylitol_ai_bridge::dto`). MUST NOT depend on
//! [`crate::protocol::wire`], `agent/`, or `infra/` implementations.

pub mod ask;
pub mod bash;
pub mod event;
pub mod export;
pub mod hook;
pub mod model;
pub mod permission;
pub mod secret;
pub mod session;
pub mod session_list;
pub mod todo;
pub mod tool;

// Flat re-exports for the common case where callers import the boundary
// trait directly from `protocol::ports` (or via `protocol` root) rather than
// from its submodule.
pub use ask::{AskArgs, AskModeArg, AskOptionArg, AskQuestionArg, AskUserGateway};
pub use bash::{BashExecOpts, XyBashExecutor, XyBashResult};
pub use event::{LifecycleHandler, XyEventSink};
pub use export::XyExportIo;
pub use hook::{NoopHookBus, XyHookBus, XyHookOutcome};
pub use model::{XyGenerateOptions, XyModel, XyModelBuilder, XyStream};
pub use permission::{XyPermission, XyPermissionVerdict};
pub use secret::XySecretResolver;
pub use session::{SessionListEntry, XySessionStore, sanitize_session_display_name};
pub use session_list::{flatten_session_forest, format_session_age};
pub use todo::AgentTodoGateway;
pub use tool::{XyBatchMode, XyTool, XyToolCtx, XyToolExecutionMode};
