//! Execution authority: ReAct loop, trust, resource discovery, and [`XyDriver`].
//!
//! Depends only on `xylitol-plugin-sdk`. Does not wire plugins and does not
//! speak the RPC envelope.

pub mod agent;
pub mod driver;
pub mod permission;
pub mod resource;
pub mod trust;
pub(crate) mod utils;

pub use agent::{
    AgentBuilder, AgentRuntime, HookBlockedError, MCP_FIRST_TURN_GATE_TIMEOUT, ModelRegistry,
    QueueMode, QueueStats, RuntimeBaseline, SessionStats, XyEvent, XyEventStream,
    max_turns_stop_hook,
};
pub use driver::{
    AttachObserve, BangExecHandler, BashRun, LinkHealth, SessionExporter, XyDriver, XyDriverError,
    XyInProcessDriver,
};
pub use permission::{
    AllowAllPermission, FilesystemRules, GlobPolicy, NetworkRules, PermissionRules, ProcessRules,
    allow_all_permission, build_permission,
};
pub use resource::DefaultResourceLoader;
pub use trust::{
    DefaultProjectTrust, TrustManager, TrustOption, TrustReason, TrustResolution,
    format_trust_prompt, prompt_trust_options_stdio, resolve_project_trusted,
};
pub use xylitol_plugin_sdk::lifecycle::XyEventError;
pub use xylitol_plugin_sdk::model::{ContextTokenEstimate, TokenProvenance};
pub use xylitol_plugin_sdk::session::TodoList;
pub use xylitol_plugin_sdk::tool_name::{BuiltinToolName, is_mcp_tool_name};
