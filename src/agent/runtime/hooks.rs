//! Agent hooks — extension points for customizing the ReAct loop.
//!
//! [`AgentHooks`] carries callback chains injected at tool-call boundaries and
//! context transforms, plus an optional pi-aligned [`ShouldStopAfterTurnHook`]
//! slot. The loop consults them when non-empty; empty chains are a cheap
//! `is_empty()` check. This is the "open for extension" seam of the runtime.
//!
//! Steering / follow-up injection is owned by [`crate::agent::session::PendingMessageQueue`]
//! on [`crate::agent::session::AgentCapabilities`] (c461). [`SteeringHooks`] remains as an
//! optional external message-source adapter and is **not** wired into the ReAct
//! loop; product paths must use `AgentCapabilities::steer` / `AgentCapabilities::follow_up` (via XyDriver).

use std::sync::Arc;

use crate::domain::message::AgentMessage;
use serde_json::Value;

// ── Hook callback type aliases ──────────────────────────────────────

/// Before-tool callback. Receives `(tool_name, call_id, arguments)`. Returning
/// `Some(reason)` denies the call; the loop records a tool-error result for
/// the LLM instead of executing the tool.
pub type BeforeToolHook = Arc<dyn Fn(&str, &str, &Value) -> Option<String> + Send + Sync>;

/// After-tool callback. Receives `(tool_name, call_id, result, is_error)`.
/// Returning `Some((new_result, new_is_error))` replaces the result seen by
/// the loop and recorded in history.
pub type AfterToolHook =
    Arc<dyn Fn(&str, &str, Value, bool) -> Option<(Value, bool)> + Send + Sync>;

/// Context-transform callback. Receives the message history before the model
/// call and returns the transformed history.
pub type TransformCtxHook = Arc<dyn Fn(Vec<AgentMessage>) -> Vec<AgentMessage> + Send + Sync>;

/// Message-injection callback (steering / follow-up). Takes no arguments and
/// returns messages to prepend to the turn.
pub type GetMessagesHook = Arc<dyn Fn() -> Vec<AgentMessage> + Send + Sync>;

/// Context passed to [`ShouldStopAfterTurnHook`] after each `TurnEnd` (pi-aligned).
#[derive(Debug, Clone)]
pub struct ShouldStopAfterTurnCtx {
    /// Zero-based turn index that just completed.
    pub turn_index: u32,
    /// Assistant message persisted for this turn, if any (pi: `message`).
    pub assistant: Option<AgentMessage>,
    /// Tool-result messages appended during this turn (may be empty).
    pub tool_results: Vec<AgentMessage>,
    /// Full conversation history after this turn's writes (pi: `context` messages).
    pub history: Vec<AgentMessage>,
    /// Messages appended by this `run` invocation so far (pi: `newMessages`).
    ///
    /// Includes the prompt user message and everything written after it; excludes
    /// `seeded_history` that existed before this `run` started.
    pub new_messages: Vec<AgentMessage>,
}

/// After-turn stop callback (pi `shouldStopAfterTurn`).
///
/// Called after `TurnEnd`. Returning `true` ends the run with `AgentEnd` without
/// draining steer / follow-up or starting another model call.
///
/// Contract: must not panic. Prefer returning `false` on uncertainty.
pub type ShouldStopAfterTurnHook = Arc<dyn Fn(&ShouldStopAfterTurnCtx) -> bool + Send + Sync>;

// ── AgentHooks ──────────────────────────────────────────────────────

/// Hooks for customizing the agent loop around tool execution and turn stop.
#[derive(Default, Clone)]
pub struct AgentHooks {
    pub before_tool_call: Vec<BeforeToolHook>,
    pub after_tool_call: Vec<AfterToolHook>,
    pub transform_context: Vec<TransformCtxHook>,
    /// Optional single-slot stop gate (pi: one callback, not a chain).
    pub should_stop_after_turn: Option<ShouldStopAfterTurnHook>,
}

impl AgentHooks {
    /// Create an empty hook set.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add a before-tool hook. Hooks run in registration order.
    pub fn add_before(&mut self, hook: BeforeToolHook) {
        self.before_tool_call.push(hook);
    }

    /// Add an after-tool hook. Hooks run in registration order.
    pub fn add_after(&mut self, hook: AfterToolHook) {
        self.after_tool_call.push(hook);
    }

    /// Add a context-transform hook.
    pub fn add_transform_context(&mut self, hook: TransformCtxHook) {
        self.transform_context.push(hook);
    }

    /// Set or clear the pi-aligned after-turn stop callback (single slot).
    pub fn set_should_stop_after_turn(&mut self, hook: Option<ShouldStopAfterTurnHook>) {
        self.should_stop_after_turn = hook;
    }
}

// ── SteeringHooks ───────────────────────────────────────────────────

/// Optional external message-source adapter (steering / follow-up).
///
/// **Not wired into the ReAct loop.** The authoritative path is
/// [`crate::agent::session::PendingMessageQueue`] on the session
/// [`crate::agent::session::AgentCapabilities`]
/// (`steer` / `follow_up` / XyDriver APIs). Keep this type only if an extension
/// needs a pull-based message source; do not dual-wire both paths.
#[derive(Default)]
pub struct SteeringHooks {
    pub get_steering_messages: Option<GetMessagesHook>,
    pub get_follow_up_messages: Option<GetMessagesHook>,
}
