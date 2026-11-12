use serde_json::{Value, json};

pub type ScriptHookCtx = (&'static str, &'static str, Value);

pub fn turn_start() -> ScriptHookCtx {
    ("turn_start", "", json!({}))
}

pub fn turn_end() -> ScriptHookCtx {
    ("turn_end", "", json!({}))
}

pub fn turn_settled() -> ScriptHookCtx {
    ("turn_settled", "", json!({}))
}

pub fn step_start(step_index: u32) -> ScriptHookCtx {
    ("step_start", "", json!({ "step_index": step_index }))
}

pub fn step_end(step_index: u32) -> ScriptHookCtx {
    ("step_end", "", json!({ "step_index": step_index }))
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
    fn step_start_payload_shape() {
        let (ty, phase, ctx) = step_start(3);
        assert_eq!(ty, "step_start");
        assert_eq!(phase, "");
        assert_eq!(ctx["step_index"], 3);
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
