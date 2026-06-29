//! Runtime protocol — boundary traits between [`crate::agent`] and [`crate::infra`].
//!
//! This layer contains only trait contracts and their signature-only
//! associated types. It depends on [`crate::domain`] for pure data types
//! and errors, and it must never depend on concrete `agent/` or `infra/`
//! implementations.

pub mod bash;
pub mod event;
pub mod export;
pub mod model;
pub mod resource;
pub mod sandbox;
pub mod secret;
pub mod session;
pub mod tool;
pub mod trust;

// Flat re-exports for the common case where callers import the boundary
// trait directly from `runtime_protocol` rather than from its submodule.
pub use bash::{BashExecutor, BashResult};
pub use event::{EventSink, LifecycleEvent, LifecycleHandler};
pub use export::ExportIo;
pub use model::{ModelBuilder, XyModel, XyStream};
pub use resource::ResourceLoader;
pub use sandbox::{SandboxEngine, SandboxVerdict};
pub use secret::SecretResolver;
pub use session::SessionStore;
pub use tool::{ToolExecutionMode, XyTool, XyToolCtx};
pub use trust::TrustStore;
