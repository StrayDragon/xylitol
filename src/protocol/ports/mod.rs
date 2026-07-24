//! Ports — boundary traits between [`crate::agent`] and [`crate::infra`].
//!
//! Trait contracts and signature-only associated types. Depends on protocol-root
//! shared types (and MAY depend on `xylitol_ai_bridge::dto`). MUST NOT depend on
//! [`crate::protocol::wire`], `agent/`, or `infra/` implementations.

pub mod bash;
pub mod event;
pub mod export;
pub mod hook;
pub mod model;
pub mod permission;
pub mod reload;
pub mod resource;
pub mod secret;
pub mod session;
pub mod session_list;
pub mod tool;
pub mod trust;

// Flat re-exports for the common case where callers import the boundary
// trait directly from `protocol::ports` (or via `protocol` root) rather than
// from its submodule.
pub use bash::{BashExecOpts, XyBashExecutor, XyBashResult};
pub use event::{LifecycleHandler, XyEventSink};
pub use export::XyExportIo;
pub use hook::{NoopHookBus, XyHookBus, XyHookOutcome};
pub use model::{XyGenerateOptions, XyModel, XyModelBuilder, XyStream};
pub use permission::{XyPermission, XyPermissionVerdict};
pub use reload::XyReloadable;
pub use resource::XyResourceLoader;
pub use secret::XySecretResolver;
pub use session::{SessionListEntry, XySessionStore, sanitize_session_display_name};
pub use session_list::{flatten_session_forest, format_session_age};
pub use tool::{XyBatchMode, XyTool, XyToolCtx, XyToolExecutionMode};
pub use trust::XyTrustStore;
