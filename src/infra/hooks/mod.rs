//! Hook event system — event-driven extension mechanism.
//!
//! Provides [`HookDispatcher`] which dispatches [`HookEvent`]s to registered
//! hook scripts, using a JSON-over-stdin/stdout protocol. Supports block/allow/
//! modify control directives with timeout handling.
//!
//! Built-in, config-gated (empty hooks list = no-op).

pub(crate) mod dispatcher;
pub(crate) mod script;

pub(crate) use dispatcher::HookDispatcher;

// ---------------------------------------------------------------------------
// HookPhase
// ---------------------------------------------------------------------------

/// Whether the hook fires before or after an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HookPhase {
    /// Before the event (can block/modify).
    Pre,
    /// After the event (observation only).
    Post,
}

impl HookPhase {
    /// Parse a phase string. Accepts "pre", "post", or empty (matches both).
    pub(crate) fn matches(&self, pattern: &str) -> bool {
        match pattern {
            "pre" => matches!(self, HookPhase::Pre),
            "post" => matches!(self, HookPhase::Post),
            "" => true, // empty = no phase filter = matches either
            _ => false,
        }
    }

    /// Return the string representation.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            HookPhase::Pre => "pre",
            HookPhase::Post => "post",
        }
    }
}

// ---------------------------------------------------------------------------
// HookEvent
// ---------------------------------------------------------------------------

/// Events that can trigger hook scripts.
///
/// Each variant carries the context data relevant to that event.
#[derive(Debug, Clone)]
pub(crate) enum HookEvent {
    /// A tool is about to be called (pre) or has completed (post).
    ToolCall {
        tool: String,
        args: serde_json::Value,
    },
    /// An LSP query tool call (pre/post).
    ToolCallLspQuery { file: String, method: String },
    /// A debugger command tool call (pre/post). (DAP PAUSED — not emitted.)
    ToolCallDapCommand { command: String },
    /// A file write tool call (pre/post).
    ToolCallFileWrite {
        file: String,
        content_preview: String,
    },
    /// About to query the model (pre only).
    ModelQuery { model: String, prompt_length: usize },
    /// A single agent step completed (post).
    StepComplete { step: u32, summary: String },
    /// Planner generated a plan (post).
    PlanGenerated { plan: String },
    /// Review phase started (pre) or ended (post).
    ReviewStart { diffs: Vec<String> },
    /// Review phase ended.
    ReviewEnd { approved: bool },
    /// Repeat detection triggered (post).
    RepeatDetected {
        loop_fragment: String,
        tokens: usize,
    },
    /// A step is being retried (pre only).
    StepRetry { retry_count: u8, reason: String },
    /// A tool call was blocked by security policy (post).
    ToolCallBlocked {
        tool: String,
        reason: String,
        rule: String,
    },
    /// Session snapshot created (post).
    SessionSnapshot { snapshot_id: String },
    /// About to spawn a new session from a snapshot (pre).
    SessionSpawn { parent_id: String },
    /// Session snapshot merged (post).
    SessionMerge { snapshot_id: String },
}

impl HookEvent {
    /// Return the event type string used in pattern matching (e.g., "tool_call",
    /// "step_complete", "model_query").
    pub(crate) fn event_type(&self) -> &'static str {
        match self {
            HookEvent::ToolCall { .. } => "tool_call",
            HookEvent::ToolCallLspQuery { .. } => "tool_call.lsp_query",
            HookEvent::ToolCallDapCommand { .. } => "tool_call.dap_command",
            HookEvent::ToolCallFileWrite { .. } => "tool_call.file_write",
            HookEvent::ModelQuery { .. } => "model_query",
            HookEvent::StepComplete { .. } => "step_complete",
            HookEvent::PlanGenerated { .. } => "plan_generated",
            HookEvent::ReviewStart { .. } => "review_start",
            HookEvent::ReviewEnd { .. } => "review_end",
            HookEvent::RepeatDetected { .. } => "repeat_detected",
            HookEvent::StepRetry { .. } => "step_retry",
            HookEvent::ToolCallBlocked { .. } => "tool_call_blocked",
            HookEvent::SessionSnapshot { .. } => "session_snapshot",
            HookEvent::SessionSpawn { .. } => "session_spawn",
            HookEvent::SessionMerge { .. } => "session_merge",
        }
    }

    /// Serialize the event to a JSON map for the hook script stdin.
    pub(crate) fn to_json_context(&self, phase: HookPhase) -> serde_json::Value {
        let mut ctx = serde_json::json!({
            "event": self.event_type(),
            "phase": phase.as_str(),
        });

        if let Some(map) = ctx.as_object_mut() {
            match self {
                HookEvent::ToolCall { tool, args } => {
                    map.insert("tool".into(), serde_json::json!(tool));
                    map.insert("args".into(), args.clone());
                }
                HookEvent::ToolCallLspQuery { file, method } => {
                    map.insert("file".into(), serde_json::json!(file));
                    map.insert("method".into(), serde_json::json!(method));
                }
                HookEvent::ToolCallDapCommand { command } => {
                    map.insert("command".into(), serde_json::json!(command));
                }
                HookEvent::ToolCallFileWrite {
                    file,
                    content_preview,
                } => {
                    map.insert("file".into(), serde_json::json!(file));
                    map.insert("content_preview".into(), serde_json::json!(content_preview));
                }
                HookEvent::ModelQuery {
                    model,
                    prompt_length,
                } => {
                    map.insert("model".into(), serde_json::json!(model));
                    map.insert("prompt_length".into(), serde_json::json!(prompt_length));
                }
                HookEvent::StepComplete { step, summary } => {
                    map.insert("step".into(), serde_json::json!(step));
                    map.insert("summary".into(), serde_json::json!(summary));
                }
                HookEvent::PlanGenerated { plan } => {
                    map.insert("plan".into(), serde_json::json!(plan));
                }
                HookEvent::ReviewStart { diffs } => {
                    map.insert("diffs".into(), serde_json::json!(diffs));
                }
                HookEvent::ReviewEnd { approved } => {
                    map.insert("approved".into(), serde_json::json!(approved));
                }
                HookEvent::RepeatDetected {
                    loop_fragment,
                    tokens,
                } => {
                    map.insert("loop_fragment".into(), serde_json::json!(loop_fragment));
                    map.insert("tokens".into(), serde_json::json!(tokens));
                }
                HookEvent::StepRetry {
                    retry_count,
                    reason,
                } => {
                    map.insert("retry_count".into(), serde_json::json!(retry_count));
                    map.insert("reason".into(), serde_json::json!(reason));
                }
                HookEvent::ToolCallBlocked { tool, reason, rule } => {
                    map.insert("tool".into(), serde_json::json!(tool));
                    map.insert("reason".into(), serde_json::json!(reason));
                    map.insert("rule".into(), serde_json::json!(rule));
                }
                HookEvent::SessionSnapshot { snapshot_id } => {
                    map.insert("snapshot_id".into(), serde_json::json!(snapshot_id));
                }
                HookEvent::SessionSpawn { parent_id } => {
                    map.insert("parent_id".into(), serde_json::json!(parent_id));
                }
                HookEvent::SessionMerge { snapshot_id } => {
                    map.insert("snapshot_id".into(), serde_json::json!(snapshot_id));
                }
            }
        }

        ctx
    }
}

// ---------------------------------------------------------------------------
// Event pattern matching
// ---------------------------------------------------------------------------

/// Check whether a hook entry's event patterns match the given event + phase.
///
/// Pattern format: `[{phase}.]{event_type}[.{qualifier}]`
/// Examples:
/// - `pre.tool_call` — matches ToolCall events in Pre phase
/// - `post.step_complete` — matches StepComplete events in Post phase
/// - `tool_call` — matches ToolCall events in either phase
/// - `pre.tool_call.bash` — matches ToolCall with tool="bash" in Pre phase
pub(crate) fn event_matches(
    entry: &crate::infra::config::types::HookEntry,
    event: &HookEvent,
    phase: HookPhase,
) -> bool {
    // Phase filter in HookEntry
    if !entry.phase.is_empty() && !phase.matches(&entry.phase) {
        return false;
    }

    let event_type = event.event_type();

    for pattern in &entry.events {
        if pattern_matches(pattern, event_type, phase, event) {
            return true;
        }
    }

    false
}

/// Check if a single pattern string matches the given event and phase.
fn pattern_matches(
    pattern: &str,
    event_type: &str,
    phase: HookPhase,
    event: &HookEvent,
) -> bool {
    let parts: Vec<&str> = pattern.split('.').collect();

    match parts.len() {
        1 => parts[0] == event_type,
        2 => parts[0] == phase.as_str() && parts[1] == event_type,
        3 => {
            parts[0] == phase.as_str()
                && parts[1] == event_type
                && matches!(event, HookEvent::ToolCall { tool, .. } if tool == parts[2])
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// HookAction
// ---------------------------------------------------------------------------

/// The result of executing a single hook.
#[derive(Debug, Clone)]
pub(crate) enum HookAction {
    /// Allow the operation to proceed.
    Allow,
    /// Block the operation with a reason.
    Block { reason: String },
    /// Modify the operation arguments and proceed.
    Modify { args: serde_json::Value },
}

impl HookAction {
    /// Parse a `HookAction` from the JSON output of a hook script.
    pub(crate) fn from_json(value: &serde_json::Value) -> Self {
        let action = value
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("allow");

        match action {
            "block" => {
                let reason = value
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Blocked by hook")
                    .to_string();
                HookAction::Block { reason }
            }
            "modify" => {
                let args = value
                    .get("args")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                HookAction::Modify { args }
            }
            _ => HookAction::Allow,
        }
    }
}

// ---------------------------------------------------------------------------
// DispatchResult
// ---------------------------------------------------------------------------

/// The aggregate result of dispatching an event to all matching hooks.
#[derive(Debug, Clone)]
pub(crate) enum DispatchResult {
    /// All hooks allowed the operation.
    Allowed,
    /// A hook blocked the operation.
    Blocked { reason: String },
    /// A hook modified the operation arguments.
    Modified { args: serde_json::Value },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── HookPhase ────────────────────────────────────────────────────

    #[test]
    fn test_phase_pre_matches_pre() {
        assert!(HookPhase::Pre.matches("pre"));
    }

    #[test]
    fn test_phase_pre_does_not_match_post() {
        assert!(!HookPhase::Pre.matches("post"));
    }

    #[test]
    fn test_phase_empty_matches_both() {
        assert!(HookPhase::Pre.matches(""));
        assert!(HookPhase::Post.matches(""));
    }

    // ── HookEvent::event_type ────────────────────────────────────────

    #[test]
    fn test_event_type_tool_call() {
        let event = HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        };
        assert_eq!(event.event_type(), "tool_call");
    }

    #[test]
    fn test_event_type_step_complete() {
        let event = HookEvent::StepComplete {
            step: 1,
            summary: "done".into(),
        };
        assert_eq!(event.event_type(), "step_complete");
    }

    #[test]
    fn test_event_type_model_query() {
        let event = HookEvent::ModelQuery {
            model: "gpt-4o".into(),
            prompt_length: 100,
        };
        assert_eq!(event.event_type(), "model_query");
    }

    // ── event_matches ────────────────────────────────────────────────

    fn make_entry(events: Vec<&str>, phase: &str) -> crate::infra::config::types::HookEntry {
        crate::infra::config::types::HookEntry {
            events: events.into_iter().map(String::from).collect(),
            phase: phase.into(),
            ..Default::default()
        }
    }

    #[test]
    fn test_match_bare_event_type() {
        let entry = make_entry(vec!["tool_call"], "");
        let event = HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        };
        assert!(event_matches(&entry, &event, HookPhase::Pre));
        assert!(event_matches(&entry, &event, HookPhase::Post));
    }

    #[test]
    fn test_match_phase_event() {
        let entry = make_entry(vec!["pre.tool_call"], "");
        let event = HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        };
        assert!(event_matches(&entry, &event, HookPhase::Pre));
        assert!(!event_matches(&entry, &event, HookPhase::Post));
    }

    #[test]
    fn test_match_phase_event_qualifier() {
        let entry = make_entry(vec!["pre.tool_call.bash"], "");
        let bash_event = HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        };
        let read_event = HookEvent::ToolCall {
            tool: "read".into(),
            args: serde_json::json!({}),
        };
        assert!(event_matches(&entry, &bash_event, HookPhase::Pre));
        assert!(!event_matches(&entry, &read_event, HookPhase::Pre));
    }

    #[test]
    fn test_match_entry_phase_filter() {
        let entry = make_entry(vec!["tool_call"], "post");
        let event = HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        };
        assert!(!event_matches(&entry, &event, HookPhase::Pre));
        assert!(event_matches(&entry, &event, HookPhase::Post));
    }

    #[test]
    fn test_no_match_different_event() {
        let entry = make_entry(vec!["step_complete"], "");
        let event = HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        };
        assert!(!event_matches(&entry, &event, HookPhase::Pre));
    }

    // ── HookAction::from_json ────────────────────────────────────────

    #[test]
    fn test_action_allow_on_empty() {
        let val = serde_json::json!({});
        assert!(matches!(HookAction::from_json(&val), HookAction::Allow));
    }

    #[test]
    fn test_action_allow_explicit() {
        let val = serde_json::json!({"action": "allow"});
        assert!(matches!(HookAction::from_json(&val), HookAction::Allow));
    }

    #[test]
    fn test_action_block() {
        let val = serde_json::json!({"action": "block", "reason": "not allowed"});
        match HookAction::from_json(&val) {
            HookAction::Block { reason } => assert_eq!(reason, "not allowed"),
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[test]
    fn test_action_modify() {
        let val = serde_json::json!({"action": "modify", "args": {"key": "value"}});
        match HookAction::from_json(&val) {
            HookAction::Modify { args } => assert_eq!(args["key"], "value"),
            other => panic!("expected Modify, got {other:?}"),
        }
    }

    // ── HookEvent::to_json_context ───────────────────────────────────

    #[test]
    fn test_to_json_context_tool_call() {
        let event = HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({"command": "ls"}),
        };
        let ctx = event.to_json_context(HookPhase::Pre);
        assert_eq!(ctx["event"], "tool_call");
        assert_eq!(ctx["phase"], "pre");
        assert_eq!(ctx["tool"], "bash");
        assert_eq!(ctx["args"]["command"], "ls");
    }
}
