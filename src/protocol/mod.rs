//! Protocol — shared contracts for wire, ports, and cross-layer vocabulary.
//!
//! Layout (scheme B):
//! - [`wire`] — client ↔ core `Command` / `Event` (transport-agnostic)
//! - [`ports`] — agent ↔ infra replaceable traits (`XyModel`, `XyTool`, …)
//! - root modules — types that appear in port/wire signatures (`AgentMessage`,
//!   `XyEvent`, session entries, …)
//!
//! Dependency: root types MUST NOT depend on `wire`/`ports`; `wire` MUST NOT
//! depend on `ports`; `ports` MAY use root types + bridge DTO. This module
//! MUST NOT depend on `agent` or `infra`.

pub mod ports;
pub mod wire;

pub mod compaction_config;
pub mod error;
pub mod lifecycle;
pub mod message;
pub mod model_config;
pub mod resource;
pub mod session;
pub mod source_info;
pub mod tool_timeout;
pub mod types;

// Wire Command/Event at protocol root (legacy call sites).
pub use wire::{Command, Envelope, ErrorCode, Event};

// Flat port re-exports for `crate::protocol::{XyModel, …}` call sites.
pub use ports::{
    BashExecOpts, LifecycleHandler, NoopHookBus, SessionListEntry, XyBashExecutor, XyBashResult,
    XyEventSink, XyExportIo, XyGenerateOptions, XyHookBus, XyHookOutcome, XyModel, XyModelBuilder,
    XyPermission, XyPermissionVerdict, XyReloadable, XyResourceLoader, XySecretResolver,
    XySessionStore, XyStream, XyTool, XyToolCtx, XyToolExecutionMode, XyTrustStore,
    flatten_session_forest, format_session_age, sanitize_session_display_name,
};

// Shared vocabulary commonly imported from protocol root.
pub use error::{XyError, XyToolError};
pub use lifecycle::XyEvent;
pub use message::{AgentMessage, AgentPart, EnvMessage, LlmMessage};
pub use model_config::{XyModelConfig, XyModelKind};
pub use tool_timeout::{MAX_TOOL_TIMEOUT_SECS, ToolTimeout, ToolTimeoutError};
pub use types::{XyChunk, XyModelMeta, XyToolSchema};
