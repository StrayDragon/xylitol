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
            timestamp: 0,
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

/// `customType` / `custom_type` on Env CustomMessage JSON
/// (c1905 `session_env` status-bar bootstrap).
pub fn message_custom_type(msg: &Value) -> Option<&str> {
    msg.get("customType")
        .or_else(|| msg.get("custom_type"))
        .and_then(Value::as_str)
}

/// Env `role=custom` rows (incl. status-bar family `session_env`) — not user-typed chat.
pub fn is_env_custom_message(msg: &Value) -> bool {
    message_role(msg) == Some("custom")
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

/// Resolve the transcript leaf anchor from persisted entries.
///
/// Honors `hint` only when it points at an existing chain entry
/// ([`SessionEntry::anchors_transcript`]); otherwise (missing, or the hint lands
/// on bookkeeping such as a parent-less trailing `modelChange`) scans back to
/// the last chain entry so resume projection / branch assembly keep full history.
pub fn transcript_leaf_anchor(entries: &[SessionEntry], hint: Option<&str>) -> Option<String> {
    if let Some(id) = hint
        && entries
            .iter()
            .find(|e| e.entry_id() == Some(id))
            .is_some_and(SessionEntry::anchors_transcript)
    {
        return Some(id.to_string());
    }
    entries
        .iter()
        .rev()
        .find(|e| e.anchors_transcript())
        .and_then(|e| e.entry_id().map(str::to_string))
}

/// Ancestry path (root → leaf) for transcript projection.
///
/// Walks `parent_id` links from `leaf`. When the walk reaches a bookkeeping row
/// ([`SessionEntry::anchors_transcript`] == false) whose parent is missing, it
/// continues from the nearest preceding chain entry in file order: cold
/// materialize used to splice parent-less `modelChange` / `thinkingLevelChange`
/// rows into otherwise intact chains, and those seams must not truncate
/// history. A chain-participating root (message / compaction / branchSummary
/// with no parent) still terminates the walk normally, so compaction cut
/// semantics are untouched.
pub fn transcript_ancestry_ids(entries: &[SessionEntry], leaf_id: Option<&str>) -> Vec<String> {
    let Some(start) = leaf_id.map(str::to_string) else {
        return Vec::new();
    };
    let mut path = vec![start.clone()];
    let mut visited = std::collections::HashSet::new();
    visited.insert(start.clone());
    let mut cur = start;
    while let Some(entry) = entries.iter().find(|e| e.entry_id() == Some(cur.as_str())) {
        let next = match entry.parent_id() {
            Some(parent) => Some(parent.to_string()),
            None if entry.anchors_transcript() => None,
            // Bookkeeping seam: splice across to the nearest preceding chain entry.
            None => {
                let pos = entries
                    .iter()
                    .position(|e| e.entry_id() == Some(cur.as_str()))
                    .unwrap_or(0);
                entries[..pos]
                    .iter()
                    .rev()
                    .find(|e| e.anchors_transcript())
                    .and_then(|e| e.entry_id())
                    .map(str::to_string)
            }
        };
        let Some(next) = next else { break };
        if !visited.insert(next.clone()) {
            break; // cycle
        }
        path.push(next.clone());
        cur = next;
    }
    path.reverse();
    path
}

#[cfg(test)]
mod leaf_anchor_tests {
    use super::*;
    use crate::protocol::session::{EntryBase, MessageEntry, ModelChangeEntry};

    fn msg(id: &str, parent: Option<&str>) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: 0,
            },
            message: fixture_message_json("user", "hi"),
        })
    }

    fn model_change(id: &str) -> SessionEntry {
        SessionEntry::ModelChange(ModelChangeEntry {
            base: EntryBase {
                entry_type: "model_change".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 0,
            },
            provider: "fake".into(),
            model_id: "fake/m".into(),
        })
    }

    #[test]
    fn leaf_anchor_skips_trailing_parentless_bookkeeping() {
        let entries = vec![msg("u1", None), msg("a1", Some("u1")), model_change("mc")];
        assert_eq!(
            transcript_leaf_anchor(&entries, None).as_deref(),
            Some("a1"),
            "resume anchor MUST land on the last chain entry, not the bookkeeping tail"
        );
        assert!(!model_change("mc").anchors_transcript());
        assert!(msg("a1", None).anchors_transcript());
    }

    #[test]
    fn leaf_anchor_honors_chain_hint_and_ignores_metadata_hint() {
        let entries = vec![msg("u1", None), msg("a1", Some("u1")), model_change("mc")];
        assert_eq!(
            transcript_leaf_anchor(&entries, Some("a1")).as_deref(),
            Some("a1")
        );
        assert_eq!(
            transcript_leaf_anchor(&entries, Some("mc")).as_deref(),
            Some("a1")
        );
        assert_eq!(
            transcript_leaf_anchor(&entries, Some("missing")).as_deref(),
            Some("a1")
        );
        assert_eq!(transcript_leaf_anchor(&[], None), None);
    }

    #[test]
    fn ancestry_splices_across_parentless_bookkeeping_seam() {
        // Historical pollution: messages chained THROUGH a parent-less
        // modelChange. The walk must splice the seam, not truncate history.
        let entries = vec![
            msg("u1", None),
            msg("a1", Some("u1")),
            model_change("mc_seam"), // parentless, mid-chain
            msg("u2", Some("mc_seam")),
            msg("a2", Some("u2")),
        ];
        assert_eq!(
            transcript_ancestry_ids(&entries, Some("a2")),
            vec!["u1", "a1", "mc_seam", "u2", "a2"],
        );
    }

    #[test]
    fn ancestry_stops_at_legitimate_chain_root_without_splice() {
        // Compaction-style cut: a chain-participating root (message with no
        // parent) terminates the walk — no linear resurrection of earlier rows.
        let entries = vec![
            msg("old1", None),
            msg("old2", Some("old1")),
            msg("kept_root", None), // e.g. post-compaction kept root
            msg("kept_child", Some("kept_root")),
        ];
        assert_eq!(
            transcript_ancestry_ids(&entries, Some("kept_child")),
            vec!["kept_root", "kept_child"],
        );
    }
}
