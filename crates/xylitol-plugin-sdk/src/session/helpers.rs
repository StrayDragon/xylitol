use serde_json::Value;

use crate::message::{AgentMessage, EnvMessage};

use super::entries::{EntryBase, MessageEntry, SessionEntry};

#[allow(clippy::too_many_arguments)]
pub fn bash_execution_message_entry(
    bash_id: impl Into<String>,
    command: impl Into<String>,
    output: impl Into<String>,
    exit_code: Option<i32>,
    cancelled: bool,
    truncated: bool,
    full_output_path: Option<String>,
    exclude_from_context: bool,
    status: crate::message::BashExecutionStatus,
) -> SessionEntry {
    let message = AgentMessage::Env(EnvMessage::BashExecutionMessage {
        bash_id: bash_id.into(),
        command: command.into(),
        output: output.into(),
        exit_code,
        cancelled,
        truncated,
        full_output_path,
        exclude_from_context,
        status,
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

pub fn fixture_message_json(role: &str, text: &str) -> Value {
    serde_json::json!({
        "role": role,
        "content": [{ "type": "text", "text": text }],
        "timestamp": 0u64,
    })
}

pub fn message_role(msg: &Value) -> Option<&str> {
    msg.get("role").and_then(Value::as_str)
}

pub fn message_custom_type(msg: &Value) -> Option<&str> {
    msg.get("customType")
        .or_else(|| msg.get("custom_type"))
        .and_then(Value::as_str)
}

pub fn is_env_custom_message(msg: &Value) -> bool {
    message_role(msg) == Some("custom")
}

pub fn message_text(msg: &Value) -> String {
    if let Some(parts) = msg.get("content").and_then(Value::as_array) {
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

pub fn message_parts(msg: &Value) -> Option<&Vec<Value>> {
    msg.get("content").and_then(Value::as_array)
}

pub fn is_tool_call_part(part: &Value) -> bool {
    part.get("type").and_then(Value::as_str) == Some("toolCall") && part.get("name").is_some()
}

pub fn tool_call_name(part: &Value) -> Option<&str> {
    if !is_tool_call_part(part) {
        return None;
    }
    part.get("name").and_then(Value::as_str)
}

#[allow(dead_code)]
pub fn tool_call_arguments(part: &Value) -> Option<&Value> {
    if !is_tool_call_part(part) {
        return None;
    }
    part.get("arguments")
}

#[allow(dead_code)]
pub fn count_tool_calls(msg: &Value) -> usize {
    message_parts(msg)
        .map(|parts| parts.iter().filter(|p| is_tool_call_part(p)).count())
        .unwrap_or(0)
}

#[allow(dead_code)]
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

pub fn is_user_message(entry: &SessionEntry) -> bool {
    matches!(
        entry,
        SessionEntry::Message(m) if message_role(&m.message) == Some("user")
    )
}

pub fn is_assistant_message(entry: &SessionEntry) -> bool {
    matches!(
        entry,
        SessionEntry::Message(m) if message_role(&m.message) == Some("assistant")
    )
}

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
            break;
        }
        path.push(next.clone());
        cur = next;
    }
    path.reverse();
    path
}

pub fn session_fork_edge(entries: &[SessionEntry]) -> (Option<String>, Option<String>) {
    for entry in entries {
        if let SessionEntry::Header(h) = entry {
            return (h.parent_session.clone(), h.fork_at_entry_id.clone());
        }
    }
    (None, None)
}

pub(crate) fn legacy_custom_type_target(custom_type: &str) -> Option<&'static str> {
    match custom_type {
        super::workflow::LEGACY_CUSTOM_TYPE_PLAN => Some(super::workflow::CUSTOM_TYPE_WORKFLOW),
        super::checkpoint::LEGACY_CUSTOM_TYPE_PLAN_CHECKPOINT => {
            Some(super::checkpoint::CUSTOM_TYPE_TODO_CHECKPOINT)
        }
        _ => None,
    }
}

#[cfg(test)]
mod leaf_anchor_tests {
    use super::*;
    use crate::session::{EntryBase, MessageEntry, ModelChangeEntry};

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
        let entries = vec![
            msg("u1", None),
            msg("a1", Some("u1")),
            model_change("mc_seam"),
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
        let entries = vec![
            msg("old1", None),
            msg("old2", Some("old1")),
            msg("kept_root", None),
            msg("kept_child", Some("kept_root")),
        ];
        assert_eq!(
            transcript_ancestry_ids(&entries, Some("kept_child")),
            vec!["kept_root", "kept_child"],
        );
    }
}

pub fn paginate_entries(
    entries: Vec<SessionEntry>,
    limit: Option<usize>,
    before_entry_id: Option<&str>,
) -> (Vec<SessionEntry>, bool) {
    let end = match before_entry_id {
        Some(id) => match entries.iter().position(|e| e.entry_id() == Some(id)) {
            Some(pos) => pos,
            None => return (Vec::new(), false),
        },
        None => entries.len(),
    };
    let Some(limit) = limit else {
        return (entries[..end].to_vec(), false);
    };
    let start = end.saturating_sub(limit);
    let has_more = start > 0;
    (entries[start..end].to_vec(), has_more)
}

#[cfg(test)]
mod pagination_tests {
    use super::*;
    use crate::session::{EntryBase, MessageEntry};

    fn entry(id: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 0,
            },
            message: serde_json::json!({"role": "user", "content": id}),
        })
    }

    fn ids(entries: &[SessionEntry]) -> Vec<String> {
        entries
            .iter()
            .filter_map(|e| e.entry_id().map(str::to_string))
            .collect()
    }

    fn entries(n: usize) -> Vec<SessionEntry> {
        (0..n).map(|i| entry(&format!("e{i}"))).collect()
    }

    #[test]
    fn no_limit_returns_everything() {
        let all = entries(5);
        let (page, has_more) = paginate_entries(all.clone(), None, None);
        assert_eq!(ids(&page), ids(&all));
        assert!(!has_more);
    }

    #[test]
    fn limit_takes_the_newest_slice() {
        let (page, has_more) = paginate_entries(entries(500), Some(200), None);
        assert_eq!(page.len(), 200);
        assert_eq!(ids(&page)[0], "e300", "must start from the newest end");
        assert_eq!(ids(&page)[199], "e499");
        assert!(has_more);
    }

    #[test]
    fn before_entry_id_walks_backwards_without_overlap() {
        let all = entries(500);
        let (first, _) = paginate_entries(all.clone(), Some(200), None);
        let oldest = first[0].entry_id().unwrap().to_string();
        let (second, has_more) = paginate_entries(all, Some(200), Some(&oldest));
        assert_eq!(second.len(), 200);
        assert_eq!(ids(&second)[199], "e299");
        assert!(
            !ids(&second).contains(&oldest),
            "pages must not overlap at the boundary"
        );
        assert!(has_more);
    }

    #[test]
    fn final_page_reports_no_more() {
        let all = entries(500);
        let (first, _) = paginate_entries(all.clone(), Some(200), None);
        let oldest = first[0].entry_id().unwrap().to_string();
        let (second, _) = paginate_entries(all.clone(), Some(200), Some(&oldest));
        let oldest2 = second[0].entry_id().unwrap().to_string();
        let (third, has_more) = paginate_entries(all, Some(200), Some(&oldest2));
        assert_eq!(third.len(), 100);
        assert!(!has_more, "no earlier entries remain");
    }

    #[test]
    fn unknown_before_entry_id_yields_an_empty_page() {
        let (page, has_more) = paginate_entries(entries(10), Some(3), Some("missing"));
        assert!(
            page.is_empty(),
            "an unknown anchor must not guess a window, or the client would page forever"
        );
        assert!(!has_more);
    }
}
