//! Protocol — shared contracts for wire, ports, and cross-layer vocabulary.
//!
//! Layout (scheme B):
//! - [`wire`] — client ↔ core `Command` / `Event` (transport-agnostic)
//! - [`ports`] — agent ↔ infra replaceable traits (`XyModel`, `XyTool`, …)
//! - clustered root types — `model`, `session`, plus flat modules
//!   (`message`, `lifecycle`, …) that appear in port/wire signatures
//!
//! Dependency: root types MUST NOT depend on `wire`/`ports`; `wire` MUST NOT
//! depend on `ports`; `ports` MAY use root types + bridge DTO. This module
//! MUST NOT depend on `agent` or `infra`.

// wire + ports stay `pub`: they are the sanctioned shared vocabulary beside the
// crate-root `Xy*` re-exports. Flat leaf modules are `pub(crate)` — reach their
// curated types via the re-exports above / at the crate root, not by deep path.
pub mod ports;
pub mod wire;

pub(crate) mod compaction_config;
pub(crate) mod error;
pub(crate) mod lifecycle;
pub(crate) mod message;
pub(crate) mod model;
pub(crate) mod model_entry;
pub(crate) mod resource;
pub(crate) mod session;
pub(crate) mod source_info;
pub(crate) mod tool_name;
pub(crate) mod tool_timeout;

// Wire Command/Event re-exported at protocol root — the canonical application-facing path.
pub use wire::{
    ApprovalRequestedPayload, Command, DOWNLINK_METHODS, Event, HostDescribeValue,
    PROTOCOL_VERSION, QuestionRequestedPayload, RpcError, RpcMessage, RpcResult,
    SessionEventPayload, SessionResourcesPayload, SessionResyncRequiredPayload,
    SessionSubscribedPayload, is_downlink_method, is_unary_method,
};

// Flat port re-exports for `crate::protocol::{XyModel, …}` call sites.
pub use ports::{
    BashExecOpts, LifecycleHandler, NoopHookBus, SessionListEntry, XyBashExecutor, XyBashResult,
    XyBatchMode, XyEventSink, XyExportIo, XyGenerateOptions, XyHookBus, XyHookOutcome, XyModel,
    XyModelBuilder, XyPermission, XyPermissionVerdict, XySecretResolver, XySessionStore, XyStream,
    XyTool, XyToolCtx, XyToolExecutionMode, flatten_session_forest, format_session_age,
    sanitize_session_display_name,
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
pub use tool_timeout::{
    DEFAULT_TOOL_TIMEOUT_SECS, MAX_EXTERNAL_WAIT_SECS, MAX_TOOL_TIMEOUT_SECS, ToolTimeout,
    ToolTimeoutError,
};
