//! Typed builders for [`XyHookBus`] `(event_type, phase, context)` triples.
//!
//! Payload field names MUST stay aligned with
//! `infra::hooks::HookEvent::payload_context` (agent cannot import infra).
//! Keeps ReAct / session call sites free of ad-hoc `json!` maps.

use serde_json::{Value, json};

/// `(event_type, phase, context)` for [`crate::protocol::ports::XyHookBus::dispatch`].
pub type ScriptHookCtx = (&'static str, &'static str, Value);

pub fn agent_start() -> ScriptHookCtx {
    ("agent_start", "", json!({}))
}

pub fn agent_end() -> ScriptHookCtx {
    ("agent_end", "", json!({}))
}

pub fn agent_settled() -> ScriptHookCtx {
    ("agent_settled", "", json!({}))
}

pub fn turn_start(turn_index: u32) -> ScriptHookCtx {
    ("turn_start", "", json!({ "turn_index": turn_index }))
}

pub fn turn_end(turn_index: u32) -> ScriptHookCtx {
    ("turn_end", "", json!({ "turn_index": turn_index }))
}

pub fn message_start(role: &str) -> ScriptHookCtx {
    ("message_start", "", json!({ "role": role }))
}

pub fn message_end(role: &str) -> ScriptHookCtx {
    ("message_end", "", json!({ "role": role }))
}

pub fn context_pre(message_count: usize) -> ScriptHookCtx {
    ("context", "pre", json!({ "message_count": message_count }))
}

pub fn tool_call_pre(tool: &str, args: &Value) -> ScriptHookCtx {
    ("tool_call", "pre", json!({ "tool": tool, "args": args }))
}

pub fn tool_result_post(tool: &str, result: impl Into<Value>, is_error: bool) -> ScriptHookCtx {
    (
        "tool_result",
        "post",
        json!({
            "tool": tool,
            "result": result.into(),
            "is_error": is_error,
        }),
    )
}

pub fn session_start(reason: &str) -> ScriptHookCtx {
    ("session_start", "", json!({ "reason": reason }))
}

pub fn session_before_fork(entry_id: &str, position: &str) -> ScriptHookCtx {
    (
        "session_before_fork",
        "pre",
        json!({ "entry_id": entry_id, "position": position }),
    )
}

pub fn session_before_compact() -> ScriptHookCtx {
    ("session_before_compact", "pre", json!({}))
}

pub fn session_compact() -> ScriptHookCtx {
    ("session_compact", "post", json!({}))
}

pub fn user_bash(command: &str, exclude_from_context: bool, cwd: &str) -> ScriptHookCtx {
    (
        "user_bash",
        "pre",
        json!({
            "command": command,
            "exclude_from_context": exclude_from_context,
            "cwd": cwd,
        }),
    )
}

pub fn session_before_switch(reason: &str, target: &str) -> ScriptHookCtx {
    (
        "session_before_switch",
        "pre",
        json!({ "reason": reason, "target": target }),
    )
}

pub fn session_shutdown_resume(target: &str, previous: Option<&str>) -> ScriptHookCtx {
    (
        "session_shutdown",
        "",
        json!({
            "reason": "resume",
            "target": target,
            "previous": previous,
        }),
    )
}

pub fn session_before_tree(kind: &str) -> ScriptHookCtx {
    ("session_before_tree", "pre", json!({ "kind": kind }))
}

pub fn session_tree(kind: &str) -> ScriptHookCtx {
    ("session_tree", "post", json!({ "kind": kind }))
}

pub fn session_before_tree_travel(kind: &str, entry_id: &str) -> ScriptHookCtx {
    (
        "session_before_tree",
        "pre",
        json!({ "kind": kind, "entry_id": entry_id }),
    )
}

pub fn session_tree_travel(kind: &str, entry_id: &str, leaf_id: Option<&str>) -> ScriptHookCtx {
    (
        "session_tree",
        "post",
        json!({
            "kind": kind,
            "entry_id": entry_id,
            "leaf_id": leaf_id,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_start_payload_shape() {
        let (ty, phase, ctx) = turn_start(3);
        assert_eq!(ty, "turn_start");
        assert_eq!(phase, "");
        assert_eq!(ctx["turn_index"], 3);
    }

    #[test]
    fn tool_call_pre_preserves_args() {
        let args = json!({"path": "a.rs"});
        let (ty, phase, ctx) = tool_call_pre("read", &args);
        assert_eq!(ty, "tool_call");
        assert_eq!(phase, "pre");
        assert_eq!(ctx["tool"], "read");
        assert_eq!(ctx["args"]["path"], "a.rs");
    }
}
