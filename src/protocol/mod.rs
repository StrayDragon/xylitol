//! Protocol — shared contracts for wire, ports, and cross-layer vocabulary.
//!
//! Vocabulary, ports, and errors live in `xylitol-plugin-sdk`. This module
//! re-exports them and keeps the remaining wire envelope until W2.2.

pub use xylitol_plugin_sdk::{
    compaction_config, error, lifecycle, message, model, model_entry, ports, resource, session,
    source_info,
};
pub(crate) use xylitol_plugin_sdk::{tool_name, tool_timeout};

pub mod wire;

// Wire Command/Event re-exported at protocol root — the canonical application-facing path.
pub use wire::{
    ApprovalRequestedPayload, Command, DOWNLINK_METHODS, Event, HostDescribeValue,
    PROTOCOL_VERSION, QuestionRequestedPayload, RpcError, RpcMessage, RpcResult,
    SessionEventPayload, SessionResourcesPayload, SessionResyncRequiredPayload,
    SessionSubscribedPayload, ToWireEvent, is_downlink_method, is_unary_method,
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

#[cfg(test)]
mod session_fold_tests;
