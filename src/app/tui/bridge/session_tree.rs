//! Rebuild live scrollback after MessageHistory travel (c615 / c646).

use crate::domain::session_types::{
    SessionEntry, SessionTreeTravel, is_tool_call_part, message_parts, message_role, message_text,
    tool_call_name,
};
use serde_json::Value;

use super::{BashBlockStatus, UiEntry, UiModel, UiPhase};

/// Replace transcript with entries on the ancestry path to `travel.leaf_id`.
pub fn rebuild_scrollback_from_travel(
    ui_model: &mut UiModel,
    entries: &[SessionEntry],
    travel: &SessionTreeTravel,
) {
    ui_model.entries.clear();
    ui_model.phase = UiPhase::Idle;
    ui_model.status = None;
    ui_model.streaming_assistant.clear();
    ui_model.streaming_thinking.clear();
    ui_model.current_role = None;

    let leaf = travel.leaf_id.as_deref();
    let path = ancestry_path_ids(entries, leaf);
    let path_label = if path.is_empty() {
        "(root)".to_string()
    } else {
        path.iter()
            .map(|id| short_entry_id(id))
            .collect::<Vec<_>>()
            .join(" → ")
    };
    ui_model.entries.push(UiEntry::System {
        text: format!(
            "history @ {} · leaf={} · path: {path_label}",
            short_entry_id(&travel.selected_id),
            leaf.map(short_entry_id).unwrap_or("(root)")
        ),
    });

    for id in path {
        let Some(entry) = entries.iter().find(|e| e.entry_id() == Some(id.as_str())) else {
            continue;
        };
        for ui in session_entry_to_ui_entries(entry) {
            ui_model.entries.push(ui);
        }
    }
}

fn ancestry_path_ids(entries: &[SessionEntry], leaf_id: Option<&str>) -> Vec<String> {
    let Some(mut cur) = leaf_id.map(str::to_string) else {
        return Vec::new();
    };
    let mut path = vec![cur.clone()];
    while let Some(parent) = entries
        .iter()
        .find(|e| e.entry_id() == Some(cur.as_str()))
        .and_then(|e| e.parent_id())
        .map(str::to_string)
    {
        path.push(parent.clone());
        cur = parent;
    }
    path.reverse();
    path
}

/// Truncate opaque ids for the travel banner (full UUID path overflows COLS and
/// can wedge differential render / CapturedScreen in PTY E2E).
fn short_entry_id(id: &str) -> &str {
    const KEEP: usize = 8;
    if id.len() > KEEP { &id[..KEEP] } else { id }
}

/// Project one session entry into zero or more UI rows (c646: thinking ≠ text).
pub fn session_entry_to_ui_entries(entry: &SessionEntry) -> Vec<UiEntry> {
    match entry {
        SessionEntry::Message(m) => message_json_to_ui_entries(&m.base.id, &m.message),
        SessionEntry::BashExecution(b) => {
            let status = if b.cancelled {
                BashBlockStatus::Cancelled
            } else if b.exit_code.is_some_and(|c| c != 0) {
                BashBlockStatus::Error
            } else {
                BashBlockStatus::Success
            };
            vec![UiEntry::Bash {
                command: b.command.clone(),
                status,
                output: b.output.clone(),
                exclude_from_context: b.exclude_from_context,
            }]
        }
        SessionEntry::Compaction(c) => vec![UiEntry::System {
            text: format!("[compaction] {}", c.summary),
        }],
        SessionEntry::BranchSummary(b) => vec![UiEntry::System {
            text: format!("[branch] {}", b.summary),
        }],
        _ => Vec::new(),
    }
}

fn message_json_to_ui_entries(entry_id: &str, message: &Value) -> Vec<UiEntry> {
    let Some(role) = message_role(message) else {
        return Vec::new();
    };
    match role {
        "user" => vec![UiEntry::User {
            text: message_text(message),
        }],
        "assistant" => assistant_parts_to_ui(entry_id, message),
        "toolResult" | "tool" => vec![UiEntry::Tool {
            id: entry_id.to_string(),
            name: message
                .get("toolName")
                .or_else(|| message.get("tool_name"))
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string(),
            args_preview: String::new(),
            output: message_text(message),
            is_error: message
                .get("isError")
                .or_else(|| message.get("is_error"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            done: true,
        }],
        _ => {
            let text = message_text(message);
            if text.is_empty() {
                Vec::new()
            } else {
                vec![UiEntry::System { text }]
            }
        }
    }
}

fn assistant_parts_to_ui(entry_id: &str, message: &Value) -> Vec<UiEntry> {
    let Some(parts) = message_parts(message) else {
        let text = message_text(message);
        return if text.is_empty() {
            Vec::new()
        } else {
            vec![UiEntry::Assistant { text }]
        };
    };

    let mut out = Vec::new();
    for part in parts {
        let typ = part.get("type").and_then(Value::as_str);
        match typ {
            Some("thinking") => {
                if let Some(t) = part
                    .get("thinking")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    out.push(UiEntry::Thinking {
                        text: t.to_string(),
                    });
                }
            }
            Some("text") => {
                if let Some(t) = part
                    .get("text")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    out.push(UiEntry::Assistant {
                        text: t.to_string(),
                    });
                }
            }
            Some("toolCall") if is_tool_call_part(part) => {
                let name = tool_call_name(part).unwrap_or("tool");
                let args = part
                    .get("arguments")
                    .or_else(|| part.get("args"))
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                out.push(UiEntry::Tool {
                    id: part
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or(entry_id)
                        .to_string(),
                    name: name.to_string(),
                    args_preview: args,
                    output: String::new(),
                    is_error: false,
                    done: false,
                });
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session_types::{EntryBase, MessageEntry, fixture_message_json};
    use serde_json::json;

    #[test]
    fn rebuild_keeps_thinking_and_assistant_separate() {
        let entry = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: Some("u1".into()),
                timestamp: "t".into(),
            },
            message: json!({
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "step 1" },
                    { "type": "text", "text": "hello" }
                ],
                "timestamp": 0u64,
            }),
        });
        let ui = session_entry_to_ui_entries(&entry);
        assert!(
            matches!(ui.as_slice(), [
                UiEntry::Thinking { text } ,
                UiEntry::Assistant { text: reply }
            ] if text == "step 1" && reply == "hello"),
            "got: {ui:?}"
        );
    }

    #[test]
    fn message_text_skips_thinking_for_prefill() {
        let msg = json!({
            "role": "assistant",
            "content": [
                { "type": "thinking", "thinking": "secret" },
                { "type": "text", "text": "visible" }
            ],
        });
        assert_eq!(message_text(&msg), "visible");
        let _ = fixture_message_json("user", "x");
    }
}
