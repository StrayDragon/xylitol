//! Agent hooks — extension points for customizing the ReAct loop.
//!
//! [`AgentHooks`] carries optional callbacks injected at tool-call boundaries
//! and context transforms. The loop consults them when present; absent hooks
//! are no-ops. This is the "open for extension" seam of the runtime.

use crate::core::message::AgentMessage;
use serde_json::Value;

// ── Hook callback type aliases ──────────────────────────────────────

/// Type alias for hook callbacks to simplify declarations.
pub type BeforeToolHook = Box<dyn Fn(&str, &str, &Value) -> Option<String> + Send + Sync>;
pub type AfterToolHook =
    Box<dyn Fn(&str, &str, Value, bool) -> Option<(Value, bool)> + Send + Sync>;
pub type TransformCtxHook = Box<dyn Fn(Vec<AgentMessage>) -> Vec<AgentMessage> + Send + Sync>;
pub type GetMessagesHook = Box<dyn Fn() -> Vec<AgentMessage> + Send + Sync>;

// ── AgentHooks ──────────────────────────────────────────────────────

/// Hooks for customizing the agent loop.
pub struct AgentHooks {
    pub before_tool_call: Option<BeforeToolHook>,
    pub after_tool_call: Option<AfterToolHook>,
    pub transform_context: Option<TransformCtxHook>,
    pub get_steering_messages: Option<GetMessagesHook>,
    pub get_follow_up_messages: Option<GetMessagesHook>,
    pub max_retries: usize,
}

impl Default for AgentHooks {
    fn default() -> Self {
        Self {
            before_tool_call: None,
            after_tool_call: None,
            transform_context: None,
            get_steering_messages: None,
            get_follow_up_messages: None,
            max_retries: 3,
        }
    }
}

// ── Queue delivery modes ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteeringMode {
    All,
    OneAtATime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowUpMode {
    Stop,
    Continue,
}
