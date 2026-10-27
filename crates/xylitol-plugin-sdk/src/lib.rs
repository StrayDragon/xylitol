pub mod accounting;
pub mod compaction_binding;
pub mod compaction_config;
pub mod compaction_error;
pub mod compaction_settings;
pub mod compat;
pub mod dto;
pub mod error;
pub mod generate;
pub mod lifecycle;
pub mod llm_project;
pub mod loop_middleware;
pub mod message;
pub mod model;
pub mod model_entry;
pub mod obs_gate;
pub mod obs_session;
pub mod overflow;
pub mod plugin_config;
pub mod plugin_events;
mod ports;
pub mod registry;
pub mod resource;
pub mod rt;
pub mod session;
pub mod session_env;
pub mod session_prepare;
pub mod settlement;
pub mod source_info;
pub mod status_bar;
pub mod token_estimator;
pub mod tool_name;
pub mod tool_timeout;
pub mod tool_wire;

pub use accounting::{
    EstimateContextOpts, HEURISTIC_BYTES_PER_TOKEN, estimate_context, heuristic_token_count,
    total_context_tokens,
};
pub use compaction_binding::{CompactionModelAttribution, CompactionSummaryBinding};
pub use compaction_error::CompactionError;
pub use compaction_settings::{
    CompactionSettings, effective_trigger_threshold, merge_compaction_runtime,
};
pub use compat::Compat;
pub use dto::{
    ContextTokenEstimate, Diagnostic, ImageContent, PromptCacheRead, TokenProvenance,
    collect_text_parts, now_ms,
};
pub use error::{
    XyError, XyExportError, XySessionError, XySessionStoreError, XyToolError, XyTrustError,
};
pub use generate::{
    ObsSessionContext, XYLITOL_OBS_LANE_ATTR, XYLITOL_OBS_LANE_LLM, XyGenerateOptions,
    langfuse_observation_properties_from, langfuse_session_properties_from,
    xylitol_obs_lane_properties,
};
pub use lifecycle::{XyEvent, XyEventError};
pub use llm_project::project_for_llm;
pub use loop_middleware::{
    AfterModelCtx, AfterToolCtx, AfterTurnCtx, BeforeModelCtx, BeforeToolCtx, BeforeTurnCtx,
    CompactRequest, DecideContinueCtx, LoopChain, LoopControl, LoopMiddleware,
    LoopMiddlewareHandle, LoopMiddlewareRegistry, LoopStage, LoopStep,
};
pub use message::{AgentMessage, AgentPart, EnvMessage, XyLlmMessage};
pub use model::{
    THINKING_OFF, ThinkingBudgets, ThinkingConfigError, ThinkingLevelMap, XyChunk, XyModelConfig,
    XyModelKind, XyModelMeta, XyToolSchema, last_declared_thinking_level,
    resolve_configured_levels, thinking_levels_are_adjustable, validate_thinking_level_map,
};
pub use obs_gate::truncate_observation_text;
pub use obs_gate::{
    OBSERVATION_IO_FULL_MAX, ObsGateScope, ObsGateState, ObservationIoTier,
    PROVIDER_TRACE_TEXT_MAX, SpanCollectScope, observation_io_tier, provider_trace_active,
    set_observation_io_tier, set_provider_trace_active, set_tool_observation_io_tier,
    tool_observation_io_tier,
};
pub use obs_session::{
    ObsSessionScope, clear_obs_session, langfuse_generation_properties,
    langfuse_generation_properties_from, langfuse_observation_properties,
    langfuse_session_properties, obs_session_context, set_obs_session, set_obs_session_name,
};
pub use overflow::{
    assistant_same_model, error_message_is_context_overflow, is_context_overflow_assistant,
};
pub use plugin_config::EmptyPluginConfig;
pub use plugin_events::{
    AgentPreStep, HttpBeforeRequest, SessionPrepare, SessionPrepareReason, SessionPrepareResult,
    ToolPostExecute, ToolPreExecute, TrustPreLoad,
};
pub use ports::{
    AgentTodoGateway, AskArgs, AskModeArg, AskOptionArg, AskQuestionArg, AskUserGateway, BashChunk,
    BashExecOpts, BashOutputSink, FS_TOOL_TIMEOUT_SECS, HeaderBag, HttpHooks, LifecycleHandler,
    NoopCompaction, NoopHookBus, OverflowCompactOutcome, SessionListEntry, TypedTool,
    XyBashExecutor, XyBashResult, XyBatchMode, XyCompaction, XyCompactionFactory, XyEventSink,
    XyExportIo, XyHookBus, XyHookOutcome, XyModel, XyModelBuilder, XyPermission,
    XyPermissionVerdict, XySecretResolver, XySessionStore, XyStream, XyTool, XyToolCtx,
    XyToolExecutionMode, ask, bash, flatten_session_forest, format_session_age,
    noop_compaction_factory, parse_tool_args, run_after_response, run_before_headers,
    run_before_request, sanitize_session_display_name, should_compact, typed,
};
pub use registry::{HostRegistries, Registry, SkillEntry};
pub use rt::{RUTIS_LOCK, SDK_API_VERSION};
pub use session_prepare::{
    CUSTOM_TYPE_SESSION_PREPARE, SESSION_PREPARE_TIMEOUT_NOTICE, SessionPrepareHub,
    SessionPrepareSnapshot, SessionPrepareView, ensure_session_prepare_in_history,
    last_session_prepare, session_prepare_from_message, should_append_session_prepare,
};
pub use settlement::{
    ContextTokenSettlement, ContextTokenSettlementReason, settle_from_session_entries,
};
pub use status_bar::{AgentStatusBar, project_outbound};
pub use token_estimator::{EstimateOpts, FixedRequestContext, estimate_from_session_entries};
pub use tool_name::{
    BuiltinToolName, MCP_PUBLIC_DELIMITER, is_mcp_tool_name, is_provider_safe_tool_name,
    mcp_tool_armed_prefix, mcp_tool_public_name,
};
pub use tool_timeout::{
    DEFAULT_TOOL_TIMEOUT_SECS, MAX_EXTERNAL_WAIT_SECS, MAX_TOOL_TIMEOUT_SECS, ToolTimeout,
    ToolTimeoutError,
};

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn sdk_api_version_and_rutis_lock() {
        assert_eq!(SDK_API_VERSION, "0.3.0");
        assert_eq!(RUTIS_LOCK, "0.17.0");
    }
}
