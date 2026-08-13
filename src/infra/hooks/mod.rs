//! Hook event system — event-driven extension mechanism.
//!
//! Provides [`HookDispatcher`] which dispatches [`HookEvent`]s to registered
//! hook scripts, using a JSON-over-stdin/stdout protocol. Supports block/allow/
//! modify control directives with timeout handling.
//!
//! Built-in, config-gated (empty hooks list = no-op).

use strum::{EnumString, IntoStaticStr};

pub mod dispatcher;
pub mod http;
pub mod script;

pub use dispatcher::HookDispatcher;

// ---------------------------------------------------------------------------
// HookPhase
// ---------------------------------------------------------------------------

/// Whether the hook fires before or after an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum HookPhase {
    /// Before the event (can block/modify).
    Pre,
    /// After the event (observation only).
    Post,
}

impl HookPhase {
    /// Parse a phase string. Accepts "pre", "post", or empty (matches both).
    pub fn matches(&self, pattern: &str) -> bool {
        match pattern {
            "pre" => matches!(self, HookPhase::Pre),
            "post" => matches!(self, HookPhase::Post),
            "" => true, // empty = no phase filter = matches either
            _ => false,
        }
    }

    /// Return the string representation.
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

// ---------------------------------------------------------------------------
// HookEvent
// ---------------------------------------------------------------------------

/// Events that can trigger hook scripts.
///
/// Each variant carries the context data relevant to that event.
#[derive(Debug, Clone)]
pub enum HookEvent {
    /// A tool is about to be called (pre) or has completed (post).
    ToolCall {
        tool: String,
        args: serde_json::Value,
    },
    /// An LSP query tool call (pre/post).
    ToolCallLspQuery {
        file: String,
        method: String,
    },
    /// A debugger command tool call (pre/post). (DAP PAUSED — not emitted.)
    ToolCallDapCommand {
        command: String,
    },
    /// A file write tool call (pre/post).
    ToolCallFileWrite {
        file: String,
        content_preview: String,
    },
    /// About to query the model (pre only).
    ModelQuery {
        model: String,
        prompt_length: usize,
    },
    /// A single agent step completed (post).
    StepComplete {
        step: u32,
        summary: String,
    },
    /// Planner generated a plan (post).
    PlanGenerated {
        plan: String,
    },
    /// Review phase started (pre) or ended (post).
    ReviewStart {
        diffs: Vec<String>,
    },
    /// Review phase ended.
    ReviewEnd {
        approved: bool,
    },
    /// Repeat detection triggered (post).
    RepeatDetected {
        loop_fragment: String,
        tokens: usize,
    },
    /// A step is being retried (pre only).
    StepRetry {
        retry_count: u8,
        reason: String,
    },
    /// A tool call was blocked by security policy (post).
    ToolCallBlocked {
        tool: String,
        reason: String,
        rule: String,
    },
    /// Session snapshot created (post).
    SessionSnapshot {
        snapshot_id: String,
    },
    /// About to spawn a new session from a snapshot (pre).
    SessionSpawn {
        parent_id: String,
    },
    /// Session snapshot merged (post).
    SessionMerge {
        snapshot_id: String,
    },
    // ── pi-aligned provider hooks ────────────────────────────────────
    /// After default auth headers are built, before HTTP send.
    BeforeProviderHeaders {
        headers: serde_json::Value,
    },
    /// After provider JSON body is built, before HTTP send.
    BeforeProviderRequest {
        model: String,
        body: serde_json::Value,
    },
    /// After HTTP response status/headers, before stream/body consumption.
    AfterProviderResponse {
        status: u16,
        headers: serde_json::Value,
    },
    // ── pi-aligned agent / turn / message ────────────────────────────
    /// Tool execution result (post).
    ToolResult {
        tool: String,
        result: serde_json::Value,
        is_error: bool,
    },
    /// Context before a model call (pre).
    Context {
        message_count: usize,
    },
    AgentStart,
    AgentEnd,
    AgentSettled,
    TurnStart {
        turn_index: u32,
    },
    TurnEnd {
        turn_index: u32,
    },
    MessageStart {
        role: String,
    },
    MessageEnd {
        role: String,
    },
    // ── pi-aligned session ───────────────────────────────────────────
    SessionStart {
        reason: String,
    },
    SessionShutdown {
        /// Why the session is shutting down (e.g. `resume`).
        reason: Option<String>,
        /// Target session id when switching.
        target: Option<String>,
        /// Previous session id when switching.
        previous: Option<String>,
    },
    SessionBeforeCompact,
    SessionCompact,
    SessionBeforeFork {
        entry_id: String,
        position: String,
    },
    SessionBeforeSwitch {
        reason: String,
        target: Option<String>,
    },
    SessionBeforeTree {
        kind: Option<String>,
        entry_id: Option<String>,
    },
    SessionTree {
        kind: Option<String>,
        entry_id: Option<String>,
        leaf_id: Option<String>,
    },
    // ── pi-aligned model / user ────────────────────────────────────
    ModelSelect {
        model: String,
    },
    ThinkingLevelSelect {
        level: String,
    },
    UserBash {
        command: String,
        exclude_from_context: Option<bool>,
        cwd: Option<String>,
    },
}

impl HookEvent {
    /// Return the event type string used in pattern matching (e.g., "tool_call",
    /// "step_complete", "model_query").
    pub fn event_type(&self) -> &'static str {
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
            HookEvent::BeforeProviderHeaders { .. } => "before_provider_headers",
            HookEvent::BeforeProviderRequest { .. } => "before_provider_request",
            HookEvent::AfterProviderResponse { .. } => "after_provider_response",
            HookEvent::ToolResult { .. } => "tool_result",
            HookEvent::Context { .. } => "context",
            HookEvent::AgentStart => "agent_start",
            HookEvent::AgentEnd => "agent_end",
            HookEvent::AgentSettled => "agent_settled",
            HookEvent::TurnStart { .. } => "turn_start",
            HookEvent::TurnEnd { .. } => "turn_end",
            HookEvent::MessageStart { .. } => "message_start",
            HookEvent::MessageEnd { .. } => "message_end",
            HookEvent::SessionStart { .. } => "session_start",
            HookEvent::SessionShutdown { .. } => "session_shutdown",
            HookEvent::SessionBeforeCompact => "session_before_compact",
            HookEvent::SessionCompact => "session_compact",
            HookEvent::SessionBeforeFork { .. } => "session_before_fork",
            HookEvent::SessionBeforeSwitch { .. } => "session_before_switch",
            HookEvent::SessionBeforeTree { .. } => "session_before_tree",
            HookEvent::SessionTree { .. } => "session_tree",
            HookEvent::ModelSelect { .. } => "model_select",
            HookEvent::ThinkingLevelSelect { .. } => "thinking_level_select",
            HookEvent::UserBash { .. } => "user_bash",
        }
    }

    /// Payload-only JSON for [`crate::protocol::ports::XyHookBus`] (no `event`/`phase` keys).
    ///
    /// Script stdin via typed [`Self::to_json_context`] still includes `event` + `phase`.
    pub fn payload_context(&self) -> serde_json::Value {
        let mut ctx = self.to_json_context(HookPhase::Pre);
        if let Some(map) = ctx.as_object_mut() {
            map.remove("event");
            map.remove("phase");
        }
        ctx
    }

    /// Serialize the event to a JSON map for the hook script stdin.
    pub fn to_json_context(&self, phase: HookPhase) -> serde_json::Value {
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
                HookEvent::BeforeProviderHeaders { headers } => {
                    map.insert("headers".into(), headers.clone());
                }
                HookEvent::BeforeProviderRequest { model, body } => {
                    map.insert("model".into(), serde_json::json!(model));
                    map.insert("body".into(), body.clone());
                }
                HookEvent::AfterProviderResponse { status, headers } => {
                    map.insert("status".into(), serde_json::json!(status));
                    map.insert("headers".into(), headers.clone());
                }
                HookEvent::ToolResult {
                    tool,
                    result,
                    is_error,
                } => {
                    map.insert("tool".into(), serde_json::json!(tool));
                    map.insert("result".into(), result.clone());
                    map.insert("is_error".into(), serde_json::json!(is_error));
                }
                HookEvent::Context { message_count } => {
                    map.insert("message_count".into(), serde_json::json!(message_count));
                }
                HookEvent::AgentStart
                | HookEvent::AgentEnd
                | HookEvent::AgentSettled
                | HookEvent::SessionBeforeCompact
                | HookEvent::SessionCompact => {}
                HookEvent::SessionShutdown {
                    reason,
                    target,
                    previous,
                } => {
                    if let Some(reason) = reason {
                        map.insert("reason".into(), serde_json::json!(reason));
                    }
                    if let Some(target) = target {
                        map.insert("target".into(), serde_json::json!(target));
                    }
                    if let Some(previous) = previous {
                        map.insert("previous".into(), serde_json::json!(previous));
                    }
                }
                HookEvent::SessionBeforeFork { entry_id, position } => {
                    map.insert("entry_id".into(), serde_json::json!(entry_id));
                    map.insert("position".into(), serde_json::json!(position));
                }
                HookEvent::TurnStart { turn_index } | HookEvent::TurnEnd { turn_index } => {
                    map.insert("turn_index".into(), serde_json::json!(turn_index));
                }
                HookEvent::MessageStart { role } | HookEvent::MessageEnd { role } => {
                    map.insert("role".into(), serde_json::json!(role));
                }
                HookEvent::SessionStart { reason } => {
                    map.insert("reason".into(), serde_json::json!(reason));
                }
                HookEvent::SessionBeforeSwitch { reason, target } => {
                    map.insert("reason".into(), serde_json::json!(reason));
                    if let Some(target) = target {
                        map.insert("target".into(), serde_json::json!(target));
                    }
                }
                HookEvent::SessionBeforeTree { kind, entry_id } => {
                    if let Some(kind) = kind {
                        map.insert("kind".into(), serde_json::json!(kind));
                    }
                    if let Some(entry_id) = entry_id {
                        map.insert("entry_id".into(), serde_json::json!(entry_id));
                    }
                }
                HookEvent::SessionTree {
                    kind,
                    entry_id,
                    leaf_id,
                } => {
                    if let Some(kind) = kind {
                        map.insert("kind".into(), serde_json::json!(kind));
                    }
                    if let Some(entry_id) = entry_id {
                        map.insert("entry_id".into(), serde_json::json!(entry_id));
                    }
                    if let Some(leaf_id) = leaf_id {
                        map.insert("leaf_id".into(), serde_json::json!(leaf_id));
                    }
                }
                HookEvent::ModelSelect { model } => {
                    map.insert("model".into(), serde_json::json!(model));
                }
                HookEvent::ThinkingLevelSelect { level } => {
                    map.insert("level".into(), serde_json::json!(level));
                }
                HookEvent::UserBash {
                    command,
                    exclude_from_context,
                    cwd,
                } => {
                    map.insert("command".into(), serde_json::json!(command));
                    if let Some(exclude) = exclude_from_context {
                        map.insert("exclude_from_context".into(), serde_json::json!(exclude));
                    }
                    if let Some(cwd) = cwd {
                        map.insert("cwd".into(), serde_json::json!(cwd));
                    }
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
pub fn event_matches(
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

/// Check whether a hook entry matches a raw event type + phase + JSON context.
///
/// Used by [`HookDispatcher::dispatch_raw`] for pi-aligned event names and
/// ReAct script bridging.
pub fn entry_matches_raw(
    entry: &crate::infra::config::types::HookEntry,
    event_type: &str,
    phase: &str,
    context: &serde_json::Value,
) -> bool {
    if !entry.phase.is_empty() && !phase_str_matches(phase, &entry.phase) {
        return false;
    }

    for pattern in &entry.events {
        if pattern_matches_raw(pattern, event_type, phase, context) {
            return true;
        }
    }

    false
}

fn phase_str_matches(actual: &str, filter: &str) -> bool {
    match filter {
        "pre" => actual == "pre",
        "post" => actual == "post",
        "" => true,
        _ => false,
    }
}

fn pattern_matches_raw(
    pattern: &str,
    event_type: &str,
    phase: &str,
    context: &serde_json::Value,
) -> bool {
    let parts: Vec<&str> = pattern.split('.').collect();

    match parts.len() {
        1 => parts[0] == event_type,
        2 => parts[0] == phase && parts[1] == event_type,
        3 => {
            parts[0] == phase
                && parts[1] == event_type
                && context.get("tool").and_then(|v| v.as_str()) == Some(parts[2])
        }
        _ => false,
    }
}

/// Check if a single pattern string matches the given event and phase.
fn pattern_matches(pattern: &str, event_type: &str, phase: HookPhase, event: &HookEvent) -> bool {
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
pub enum HookAction {
    /// Allow the operation to proceed.
    Allow,
    /// Block the operation with a reason.
    Block { reason: String },
    /// Modify the operation arguments and proceed.
    Modify { args: serde_json::Value },
}

impl HookAction {
    /// Parse a `HookAction` from the JSON output of a hook script.
    pub fn from_json(value: &serde_json::Value) -> Self {
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
pub enum DispatchResult {
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

    #[test]
    fn test_payload_context_omits_event_phase() {
        let event = HookEvent::TurnStart { turn_index: 2 };
        let payload = event.payload_context();
        assert!(payload.get("event").is_none());
        assert!(payload.get("phase").is_none());
        assert_eq!(payload["turn_index"], 2);
    }

    #[test]
    fn test_session_before_fork_payload_fields() {
        let event = HookEvent::SessionBeforeFork {
            entry_id: "e1".into(),
            position: "After".into(),
        };
        let payload = event.payload_context();
        assert_eq!(payload["entry_id"], "e1");
        assert_eq!(payload["position"], "After");
    }

    #[test]
    fn test_event_type_before_provider_request() {
        let event = HookEvent::BeforeProviderRequest {
            model: "gpt-4o".into(),
            body: serde_json::json!({}),
        };
        assert_eq!(event.event_type(), "before_provider_request");
    }

    #[test]
    fn test_event_type_after_provider_response() {
        let event = HookEvent::AfterProviderResponse {
            status: 200,
            headers: serde_json::json!({}),
        };
        assert_eq!(event.event_type(), "after_provider_response");
    }

    #[test]
    fn test_entry_matches_raw_pi_name() {
        let entry = make_entry(vec!["before_provider_request"], "");
        assert!(entry_matches_raw(
            &entry,
            "before_provider_request",
            "pre",
            &serde_json::json!({})
        ));
    }

    #[test]
    fn test_entry_matches_raw_phase_event() {
        let entry = make_entry(vec!["pre.tool_call"], "");
        assert!(entry_matches_raw(
            &entry,
            "tool_call",
            "pre",
            &serde_json::json!({"tool": "bash"})
        ));
        assert!(!entry_matches_raw(
            &entry,
            "tool_call",
            "post",
            &serde_json::json!({})
        ));
    }
}
