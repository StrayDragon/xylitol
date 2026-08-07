//! Session message JSON helpers and bash entry builders.

use serde_json::Value;

use crate::protocol::message::{AgentMessage, EnvMessage};

use super::entries::{EntryBase, MessageEntry, SessionEntry};

/// Build a nested bash `SessionEntry::Message` for new bang writes (c1210 / be4).
pub fn bash_execution_message_entry(
    command: impl Into<String>,
    output: impl Into<String>,
    exit_code: Option<i32>,
    cancelled: bool,
    truncated: bool,
    full_output_path: Option<String>,
    exclude_from_context: bool,
) -> SessionEntry {
    let message = AgentMessage::Env(EnvMessage::BashExecutionMessage {
        command: command.into(),
        output: output.into(),
        exit_code,
        cancelled,
        truncated,
        full_output_path,
        exclude_from_context,
    });
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: String::new(),
        },
        message: serde_json::to_value(&message).unwrap_or(Value::Null),
    })
}

// ── Message entry helpers ───────────────────────────────────────────

/// AgentMessage-shaped JSON for session fixtures (c646 tagged content).
pub fn fixture_message_json(role: &str, text: &str) -> Value {
    serde_json::json!({
        "role": role,
        "content": [{ "type": "text", "text": text }],
        "timestamp": 0u64,
    })
}

/// Extract the `role` field from a serialized agent message JSON value.
pub fn message_role(msg: &Value) -> Option<&str> {
    msg.get("role").and_then(Value::as_str)
}

/// Extract **visible text** for editor prefill / tree summary (dm2).
///
/// Only aggregates `type=text` parts. Thinking / toolCall / image are skipped.
/// Bare-string content and untyped objects are ignored (c646: no legacy read).
pub fn message_text(msg: &Value) -> String {
    if let Some(parts) = msg
        .get("content")
        .or_else(|| msg.get("parts"))
        .and_then(Value::as_array)
    {
        return extract_text_from_parts(parts);
    }
    String::new()
}

fn extract_text_from_parts(parts: &[Value]) -> String {
    let mut out = String::new();
    for p in parts {
        let Some(obj) = p.as_object() else {
            continue;
        };
        if obj.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        if let Some(t) = obj.get("text").and_then(Value::as_str) {
            out.push_str(t);
        }
    }
    out
}

/// Iterate `content` or legacy `parts` arrays on a serialized message.
pub fn message_parts(msg: &Value) -> Option<&Vec<Value>> {
    msg.get("content")
        .or_else(|| msg.get("parts"))
        .and_then(Value::as_array)
}

/// Whether a content/part value is a tool-call (requires `type: toolCall`).
pub fn is_tool_call_part(part: &Value) -> bool {
    part.get("type").and_then(Value::as_str) == Some("toolCall") && part.get("name").is_some()
}

/// Tool name from a tool-call part, if any.
pub fn tool_call_name(part: &Value) -> Option<&str> {
    if !is_tool_call_part(part) {
        return None;
    }
    part.get("name").and_then(Value::as_str)
}

/// Tool arguments object (`arguments` or legacy `args`).
pub fn tool_call_arguments(part: &Value) -> Option<&Value> {
    if !is_tool_call_part(part) {
        return None;
    }
    part.get("arguments").or_else(|| part.get("args"))
}

/// Count tool-call parts in a serialized message.
pub fn count_tool_calls(msg: &Value) -> usize {
    message_parts(msg)
        .map(|parts| parts.iter().filter(|p| is_tool_call_part(p)).count())
        .unwrap_or(0)
}

/// Collect unique `path` args from read/write/edit tool calls in a message.
pub fn tool_file_paths(msg: &Value) -> Vec<String> {
    let Some(parts) = message_parts(msg) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for part in parts {
        let Some(name) = tool_call_name(part) else {
            continue;
        };
        if !matches!(name, "read" | "write" | "edit") {
            continue;
        }
        let Some(path) = tool_call_arguments(part)
            .and_then(|a| a.get("path"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if !files.iter().any(|f| f == path) {
            files.push(path.to_string());
        }
    }
    files
}

/// Whether `entry` is a persisted user message.
pub fn is_user_message(entry: &SessionEntry) -> bool {
    matches!(
        entry,
        SessionEntry::Message(m) if message_role(&m.message) == Some("user")
    )
}

/// Whether `entry` is a persisted assistant message.
pub fn is_assistant_message(entry: &SessionEntry) -> bool {
    matches!(
        entry,
        SessionEntry::Message(m) if message_role(&m.message) == Some("assistant")
    )
}
