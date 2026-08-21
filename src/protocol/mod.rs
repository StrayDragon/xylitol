//! Protocol — shared contracts for wire, ports, and cross-layer vocabulary.
//!
//! Layout (scheme B):
//! - [`wire`] — client ↔ core `Command` / `Event` (transport-agnostic)
//! - [`ports`] — agent ↔ infra replaceable traits (`XyModel`, `XyTool`, …)
//! - clustered root types — [`model`], [`session`], plus flat modules
//!   (`message`, `lifecycle`, …) that appear in port/wire signatures
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
pub mod model;
pub mod resource;
pub mod session;
pub mod source_info;
pub mod tool_name;
pub mod tool_timeout;

// Wire Command/Event at protocol root (legacy call sites).
pub use wire::{
    ApprovalRequestedPayload, BINDINGS_RELATIVE_PATH, Command, DOWNLINK_METHODS, Envelope,
    ErrorCode, Event, HostDescribeValue, HostHelloPayload, PROTOCOL_VERSION,
    QuestionRequestedPayload, RpcError, RpcMessage, RpcResult, SessionEventPayload,
    SessionResourcesPayload, SessionResyncRequiredPayload, SessionSubscribedPayload, UNARY_METHODS,
    export_typescript_bindings, is_downlink_method, is_unary_method,
};

// Flat port re-exports for `crate::protocol::{XyModel, …}` call sites.
pub use ports::{
    BashExecOpts, LifecycleHandler, NoopHookBus, SessionListEntry, XyBashExecutor, XyBashResult,
    XyBatchMode, XyEventSink, XyExportIo, XyGenerateOptions, XyHookBus, XyHookOutcome, XyModel,
    XyModelBuilder, XyPermission, XyPermissionVerdict, XyReloadable, XyResourceLoader,
    XySecretResolver, XySessionStore, XyStream, XyTool, XyToolCtx, XyToolExecutionMode,
    XyTrustStore, flatten_session_forest, format_session_age, sanitize_session_display_name,
};

// Shared vocabulary commonly imported from protocol root.
pub use error::{
    XyError, XyExportError, XySessionError, XySessionStoreError, XyToolError, XyTrustError,
};
pub use lifecycle::{XyEvent, XyEventError};
pub use message::{AgentMessage, AgentPart, EnvMessage, LlmMessage};
pub use model::{XyChunk, XyModelMeta, XyToolSchema};
pub use model::{XyModelConfig, XyModelKind};
pub use tool_name::{
    BuiltinToolName, MCP_PUBLIC_DELIMITER, is_mcp_tool_name, is_provider_safe_tool_name,
    mcp_tool_armed_prefix, mcp_tool_public_name,
};
pub use tool_timeout::{MAX_TOOL_TIMEOUT_SECS, ToolTimeout, ToolTimeoutError};
