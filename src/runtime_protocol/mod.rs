//! Runtime protocol — boundary traits between [`crate::agent`] and [`crate::infra`].
//!
//! This layer contains only trait contracts and their signature-only
//! associated types. It depends on [`crate::domain`] for pure data types
//! and errors, and it must never depend on concrete `agent/` or `infra/`
//! implementations.

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
// trait directly from `runtime_protocol` rather than from its submodule.
pub use bash::{BashExecOpts, XyBashExecutor, XyBashResult};
pub use event::{LifecycleHandler, XyEventSink};
pub use export::XyExportIo;
pub use hook::{NoopHookBus, XyHookBus, XyHookOutcome};
pub use model::{XyModel, XyModelBuilder, XyStream};
pub use permission::{XyPermission, XyPermissionVerdict};
pub use reload::XyReloadable;
pub use resource::XyResourceLoader;
pub use secret::XySecretResolver;
pub use session::{SessionListEntry, XySessionStore, sanitize_session_display_name};
pub use session_list::{flatten_session_forest, format_session_age};
pub use tool::{XyTool, XyToolCtx, XyToolExecutionMode};
pub use trust::XyTrustStore;
