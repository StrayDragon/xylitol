//! Product method table (unary names). Payload shapes stay on `Command`.

/// Unary methods registered for v1. Unknown names MUST fail envelope parse/dispatch.
pub const UNARY_METHODS: &[&str] = &[
    "prompt",
    "abort",
    "get_state",
    "set_model",
    "cycle_model",
    "get_available_models",
    "set_thinking_level",
    "bash",
    "compact",
    "get_session_stats",
    "export_html",
    "export_jsonl",
    "import_jsonl",
    "switch_session",
    "fork",
    "get_messages",
    "get_commands",
    "session_tree",
    "travel_session_tree",
    "append_entry_label",
    "list_sessions",
    "load_session_entries",
    "new_session",
    "get_session_name",
    "set_session_name",
    "set_session_name_for",
    "delete_session",
    "reload",
    "loaded_resources",
    "queue_stats",
    "load_debug_scene",
    "arm_tool_freeze",
    "persist_trust",
    "steer",
    "follow_up",
    "clear_queue",
    "subscribe",
    "host.describe",
];

/// Downlink `ServerRequest.method` values (not unary).
pub const DOWNLINK_METHODS: &[&str] = &[
    "session/event",
    "session/subscribed",
    "session/resync_required",
    "session/resources",
    "approval/requested",
    "question/requested",
    "host/hello",
];

pub fn is_unary_method(name: &str) -> bool {
    UNARY_METHODS.contains(&name)
}

pub fn is_downlink_method(name: &str) -> bool {
    DOWNLINK_METHODS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_is_not_unary() {
        assert!(!is_unary_method("quit"));
        assert!(!is_unary_method("approve_tool"));
        assert!(is_unary_method("prompt"));
        assert!(is_unary_method("host.describe"));
        assert!(is_unary_method("arm_tool_freeze"));
        assert!(is_downlink_method("session/event"));
        assert!(is_downlink_method("session/resources"));
    }
}
