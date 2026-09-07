//! Agent hooks — extension points for customizing the ReAct loop.
//!
//! [`AgentHooks`] carries callback chains injected at tool-call boundaries,
//! plus an optional pi-aligned [`ShouldStopAfterTurnHook`] slot. The loop
//! consults them when non-empty; empty chains are a cheap `is_empty()` check.
//! This is the "open for extension" seam of the runtime.
//!
//! Steering / follow-up injection is owned by [`crate::agent::capabilities::PendingMessageQueue`]
//! on [`crate::agent::capabilities::AgentCapabilities`] (c461). Product paths use
//! `AgentCapabilities::steer` / `AgentCapabilities::follow_up` (via XyDriver).

use std::sync::Arc;

use crate::protocol::message::AgentMessage;
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

/// Build a [`ShouldStopAfterTurnHook`] for `session.max_turns` (c1620 / ar30).
///
/// Stops after `max_turns` completed turns (`turn_index` is 0-based).
pub fn max_turns_stop_hook(max_turns: u32) -> ShouldStopAfterTurnHook {
    Arc::new(move |ctx: &ShouldStopAfterTurnCtx| ctx.turn_index.saturating_add(1) >= max_turns)
}

#[cfg(test)]
mod max_turns_hook_tests {
    use super::*;

    fn ctx(turn_index: u32) -> ShouldStopAfterTurnCtx {
        ShouldStopAfterTurnCtx {
            turn_index,
            assistant: None,
            tool_results: vec![],
            history: vec![],
            new_messages: vec![],
        }
    }

    #[test]
    fn stops_when_completed_turns_reach_cap() {
        let hook = max_turns_stop_hook(2);
        assert!(!hook(&ctx(0)));
        assert!(hook(&ctx(1)));
        assert!(hook(&ctx(2)));
    }
}

// ── AgentHooks ──────────────────────────────────────────────────────

/// Hooks for customizing the agent loop around tool execution and turn stop.
#[derive(Default, Clone)]
pub struct AgentHooks {
    pub before_tool_call: Vec<BeforeToolHook>,
    pub after_tool_call: Vec<AfterToolHook>,
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

    /// Set or clear the pi-aligned after-turn stop callback (single slot).
    pub fn set_should_stop_after_turn(&mut self, hook: Option<ShouldStopAfterTurnHook>) {
        self.should_stop_after_turn = hook;
    }
}
