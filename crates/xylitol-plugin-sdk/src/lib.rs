//! Plugin vocabulary: ports, errors, and shared types (no HTTP, no envelope).

pub mod accounting;
pub mod compaction_config;
pub mod compat;
pub mod dto;
pub mod error;
pub mod generate;
pub mod lifecycle;
pub mod message;
pub mod model;
pub mod model_entry;
pub mod ports;
pub mod resource;
pub mod session;
pub mod source_info;
pub mod tool_name;
pub mod tool_timeout;
pub mod tool_wire;

pub use accounting::{
    EstimateContextOpts, HEURISTIC_BYTES_PER_TOKEN, estimate_context, heuristic_token_count,
    total_context_tokens,
};
pub use compat::Compat;
pub use dto::*;
pub use error::{
    AiBridgeError, XyError, XyExportError, XySessionError, XySessionStoreError, XyToolError,
    XyTrustError,
};
pub use generate::{ObsSessionContext, XyGenerateOptions};
pub use lifecycle::{XyEvent, XyEventError};
pub use message::{AgentMessage, AgentPart, EnvMessage, LlmMessage};
pub use model::{
    THINKING_OFF, ThinkingBudgets, ThinkingConfigError, ThinkingLevelMap, XyChunk, XyModelConfig,
    XyModelKind, XyModelMeta, XyToolSchema, last_declared_thinking_level,
    resolve_configured_levels, thinking_levels_are_adjustable, validate_thinking_level_map,
};
pub use ports::{
    BashExecOpts, HeaderBag, HttpHooks, LifecycleHandler, NoopHookBus, SessionListEntry,
    XyBashExecutor, XyBashResult, XyBatchMode, XyEventSink, XyExportIo, XyHookBus, XyHookOutcome,
    XyModel, XyModelBuilder, XyPermission, XyPermissionVerdict, XySecretResolver, XySessionStore,
    XyStream, XyTool, XyToolCtx, XyToolExecutionMode, flatten_session_forest, format_session_age,
    run_after_response, run_before_headers, run_before_request, sanitize_session_display_name,
};
pub use tool_name::{
    BuiltinToolName, MCP_PUBLIC_DELIMITER, is_mcp_tool_name, is_provider_safe_tool_name,
    mcp_tool_armed_prefix, mcp_tool_public_name,
};
pub use tool_timeout::{
    DEFAULT_TOOL_TIMEOUT_SECS, MAX_EXTERNAL_WAIT_SECS, MAX_TOOL_TIMEOUT_SECS, ToolTimeout,
    ToolTimeoutError,
};

/// Historical bridge alias; same type as [`XyGenerateOptions`].
pub type AiBridgeGenerateOptions = XyGenerateOptions;
/// Historical bridge alias; same type as [`ThinkingBudgets`].
pub type AiBridgeThinkingBudgets = ThinkingBudgets;
