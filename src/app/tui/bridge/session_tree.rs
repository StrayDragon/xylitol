//! Rebuild live scrollback after MessageHistory travel (c615 / c646).

use crate::protocol::session::{
    SessionEntry, SessionTreeTravel, is_tool_call_part, message_parts, message_role, message_text,
    tool_call_name,
};
use serde_json::Value;

use super::{
    BashBlockStatus, CompactionBlockStatus, UiEntry, UiModel, UiPhase,
    apply_tool_result_to_entries, find_tool_mut,
};

/// Replace transcript with entries on the ancestry path to `travel.leaf_id`.
///
/// Path projection only — callers that need a travel notice MUST append it
/// via [`travel_history_note`] + `push_scroll_notice` (trailing, not prepend).
///
/// `toolResult` rows merge into the matching Tool by `toolCallId` (same as live End).
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

    let path = ancestry_path_ids(entries, travel.leaf_id.as_deref());
    for id in path {
        let Some(entry) = entries.iter().find(|e| e.entry_id() == Some(id.as_str())) else {
            continue;
        };
        if let SessionEntry::Message(m) = entry {
            let role = message_role(&m.message);
            if matches!(role, Some("toolResult") | Some("tool")) {
                merge_persisted_tool_result(&mut ui_model.entries, &m.base.id, &m.message);
                continue;
            }
        }
        for ui in session_entry_to_ui_entries(entry) {
            ui_model.entries.push(ui);
        }
    }
}

/// Merge a persisted toolResult into scrollback (att12): same End semantics as live.
fn merge_persisted_tool_result(entries: &mut Vec<UiEntry>, entry_id: &str, message: &Value) {
    let tool_call_id = message
        .get("toolCallId")
        .or_else(|| message.get("tool_call_id"))
        .and_then(Value::as_str)
        .unwrap_or(entry_id);
    let name = message
        .get("toolName")
        .or_else(|| message.get("tool_name"))
        .and_then(Value::as_str)
        .unwrap_or("tool");
    let is_error = message
        .get("isError")
        .or_else(|| message.get("is_error"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let result = message_text(message);
    let details_diff = message
        .get("details")
        .and_then(|d| d.get("display_diff"))
        .and_then(Value::as_str)
        .map(str::to_string);

    if name == "ask" {
        let (phase, summary, detail_lines) =
            crate::app::tui::bridge::humanize_ask_result(&result, is_error);
        if let Some(UiEntry::Ask {
            summary: s,
            detail_lines: d,
            phase: p,
            expanded,
            ..
        }) = entries.iter_mut().rev().find(|e| match e {
            UiEntry::Ask { id: tid, .. } => tid == tool_call_id,
            _ => false,
        }) {
            *s = summary;
            *d = detail_lines;
            *p = phase;
            *expanded = false;
        } else {
            entries.push(UiEntry::Ask {
                id: tool_call_id.to_string(),
                summary,
                detail_lines,
                phase,
                expanded: false,
            });
        }
        return;
    }

    if !apply_tool_result_to_entries(entries, tool_call_id, name, &result, is_error) {
        // Orphan: no matching call on path — still one done Tool row (id prefers toolCallId).
        entries.push(UiEntry::Tool {
            id: tool_call_id.to_string(),
            name: name.to_string(),
            args_preview: String::new(),
            tool_path: None,
            write_content: None,
            display_diff: None,
            output: String::new(),
            is_error: false,
            done: false,
        });
        let _ = apply_tool_result_to_entries(entries, tool_call_id, name, &result, is_error);
    }

    if let Some(diff) = details_diff
        && let Some(UiEntry::Tool {
            display_diff: slot, ..
        }) = find_tool_mut(entries, tool_call_id)
        && slot.is_none()
    {
        *slot = Some(diff);
    }
}

/// Format the trailing travel System notice (`history @ … · leaf=… · path: …`).
pub fn travel_history_note(entries: &[SessionEntry], travel: &SessionTreeTravel) -> String {
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
    format!(
        "history @ {} · leaf={} · path: {path_label}",
        short_entry_id(&travel.selected_id),
        leaf.map(short_entry_id).unwrap_or("(root)")
    )
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
        SessionEntry::Message(m) if message_role(&m.message) == Some("bashExecution") => {
            nested_bash_to_ui(&m.message)
        }
        SessionEntry::Message(m) => message_json_to_ui_entries(&m.base.id, &m.message),
        SessionEntry::Compaction(c) => vec![UiEntry::Compaction {
            status: CompactionBlockStatus::Complete,
            summary: c.summary.clone(),
            tokens_before: c.tokens_before,
            detail: None,
        }],
        SessionEntry::BranchSummary(b) => vec![UiEntry::ScrollNotice {
            text: format!("[branch] {}", b.summary),
        }],
        _ => Vec::new(),
    }
}

fn nested_bash_to_ui(message: &Value) -> Vec<UiEntry> {
    let command = message
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let output = message
        .get("output")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let cancelled = message
        .get("cancelled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let exit_code = message
        .get("exitCode")
        .or_else(|| message.get("exit_code"))
        .and_then(Value::as_i64);
    let exclude_from_context = message
        .get("excludeFromContext")
        .or_else(|| message.get("exclude_from_context"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status = if cancelled {
        BashBlockStatus::Cancelled
    } else if exit_code.is_some_and(|c| c != 0) {
        BashBlockStatus::Error
    } else {
        BashBlockStatus::Success
    };
    vec![UiEntry::Bash {
        command,
        status,
        output,
        exclude_from_context,
    }]
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
        "toolResult" | "tool" => {
            // Standalone projection (orphan / direct call). Rebuild path merges via
            // [`merge_persisted_tool_result`] instead of appending a second Tool.
            let tool_call_id = message
                .get("toolCallId")
                .or_else(|| message.get("tool_call_id"))
                .and_then(Value::as_str)
                .unwrap_or(entry_id);
            let details = message.get("details");
            let display_diff = details
                .and_then(|d| d.get("display_diff"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let name = message
                .get("toolName")
                .or_else(|| message.get("tool_name"))
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string();
            let mut entries = vec![UiEntry::Tool {
                id: tool_call_id.to_string(),
                name: name.clone(),
                args_preview: String::new(),
                tool_path: None,
                write_content: None,
                display_diff: None,
                output: String::new(),
                is_error: false,
                done: false,
            }];
            let is_error = message
                .get("isError")
                .or_else(|| message.get("is_error"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let _ = apply_tool_result_to_entries(
                &mut entries,
                tool_call_id,
                &name,
                &message_text(message),
                is_error,
            );
            if let Some(diff) = display_diff
                && let Some(UiEntry::Tool {
                    display_diff: slot, ..
                }) = find_tool_mut(&mut entries, tool_call_id)
                && slot.is_none()
            {
                *slot = Some(diff);
            }
            entries
        }
        _ => {
            let text = message_text(message);
            if text.is_empty() {
                Vec::new()
            } else {
                vec![UiEntry::ScrollNotice { text }]
            }
        }
    }
}

fn assistant_parts_to_ui(entry_id: &str, message: &Value) -> Vec<UiEntry> {
    let stop = message
        .get("stopReason")
        .or_else(|| message.get("stop_reason"))
        .and_then(Value::as_str);
    let error_message = message
        .get("errorMessage")
        .or_else(|| message.get("error_message"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());

    let Some(parts) = message_parts(message) else {
        let text = message_text(message);
        return assistant_terminal_ui(stop, error_message, text);
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
                use crate::app::tool_display::{is_mcp_tool_name, mcp_tool_body};

                let name = tool_call_name(part).unwrap_or("tool");
                let id = part
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or(entry_id)
                    .to_string();
                if name == "ask" {
                    out.push(UiEntry::Ask {
                        id,
                        summary: "Ask · 等待回答…".into(),
                        detail_lines: vec![],
                        phase: crate::app::tui::bridge::AskPhase::Waiting,
                        expanded: false,
                    });
                    continue;
                }
                let args = part
                    .get("arguments")
                    .or_else(|| part.get("args"))
                    .cloned()
                    .unwrap_or(Value::Object(Default::default()));
                let mcp = is_mcp_tool_name(name);
                out.push(UiEntry::Tool {
                    id,
                    name: name.to_string(),
                    args_preview: if mcp {
                        String::new()
                    } else {
                        crate::app::tui::bridge::human_tool_args_preview(name, &args, usize::MAX)
                    },
                    tool_path: crate::app::tui::bridge::extract_tool_path(&args),
                    write_content: (name == "write")
                        .then(|| {
                            args.get("content")
                                .and_then(Value::as_str)
                                .map(str::to_string)
                        })
                        .flatten()
                        .filter(|s| !s.is_empty()),
                    display_diff: None,
                    output: if mcp {
                        mcp_tool_body(Some(&args), None)
                    } else {
                        String::new()
                    },
                    is_error: false,
                    done: false,
                });
            }
            _ => {}
        }
    }

    push_assistant_terminal_note(&mut out, stop, error_message);
    out
}

/// Map stopReason/errorMessage into scrollback rows (pi rebuild + live Error/abort).
fn assistant_terminal_ui(
    stop: Option<&str>,
    error_message: Option<&str>,
    text: String,
) -> Vec<UiEntry> {
    let mut out = Vec::new();
    if !text.is_empty() {
        out.push(UiEntry::Assistant { text });
    }
    push_assistant_terminal_note(&mut out, stop, error_message);
    out
}

fn push_assistant_terminal_note(
    out: &mut Vec<UiEntry>,
    stop: Option<&str>,
    error_message: Option<&str>,
) {
    match stop {
        Some("error") => {
            let text = error_message
                .map(str::to_string)
                .unwrap_or_else(|| "Error".into());
            out.push(UiEntry::Error { text });
        }
        Some("aborted") => {
            let text = error_message
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| "Operation aborted".into());
            out.push(UiEntry::ScrollNotice { text });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::{EntryBase, MessageEntry, fixture_message_json};
    use serde_json::json;

    #[test]
    fn rebuild_projects_path_only_without_history_banner() {
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: "t".into(),
                },
                message: fixture_message_json("user", "hi"),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: "t".into(),
                },
                message: fixture_message_json("assistant", "yo"),
            }),
        ];
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: "a1".into(),
            leaf_id: Some("a1".into()),
            editor_text: None,
        };
        let mut ui = UiModel::default();
        rebuild_scrollback_from_travel(&mut ui, &entries, &travel);
        assert!(
            ui.entries.iter().all(
                |e| !matches!(e, UiEntry::ScrollNotice { text } if text.contains("history @"))
            ),
            "rebuild MUST NOT insert history @; got: {:?}",
            ui.entries
        );
        assert!(
            matches!(
                ui.entries.as_slice(),
                [
                    UiEntry::User { text: u },
                    UiEntry::Assistant { text: a }
                ] if u == "hi" && a == "yo"
            ),
            "got: {:?}",
            ui.entries
        );
        let note = travel_history_note(&entries, &travel);
        assert!(
            note.contains("history @ a1") && note.contains("path:"),
            "{note}"
        );
    }

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
    fn compaction_entry_maps_to_collapsed_block() {
        let entry = SessionEntry::Compaction(crate::protocol::session::CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: Some("u1".into()),
                timestamp: "t".into(),
            },
            summary: "## Goal\nkeep going".into(),
            first_kept_entry_id: "u2".into(),
            tokens_before: 186_842,
            details: None,
            from_hook: None,
        });
        let ui = session_entry_to_ui_entries(&entry);
        assert!(
            matches!(
                ui.as_slice(),
                [UiEntry::Compaction {
                    status: CompactionBlockStatus::Complete,
                    tokens_before: 186_842,
                    summary,
                    detail: None,
                }] if summary.contains("keep going")
            ),
            "got: {ui:?}"
        );
    }

    #[test]
    fn error_assistant_empty_content_surfaces_error_row() {
        let entry = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: Some("u1".into()),
                timestamp: "t".into(),
            },
            message: json!({
                "role": "assistant",
                "content": [{ "type": "text", "text": "" }],
                "stopReason": "error",
                "errorMessage": "request exceeds the available context size",
                "timestamp": 0u64,
            }),
        });
        let ui = session_entry_to_ui_entries(&entry);
        assert!(
            matches!(
                ui.as_slice(),
                [UiEntry::Error { text }] if text.contains("exceeds the available context size")
            ),
            "got: {ui:?}"
        );
    }

    #[test]
    fn error_assistant_accepts_legacy_snake_stop_reason() {
        let entry = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "a1".into(),
                parent_id: Some("u1".into()),
                timestamp: "t".into(),
            },
            message: json!({
                "role": "assistant",
                "content": [{ "type": "text", "text": "" }],
                "stop_reason": "error",
                "error_message": "legacy overflow",
                "timestamp": 0u64,
            }),
        });
        let ui = session_entry_to_ui_entries(&entry);
        assert!(
            matches!(ui.as_slice(), [UiEntry::Error { text }] if text == "legacy overflow"),
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

    #[test]
    fn rebuild_merges_tool_call_and_result_into_one_tool() {
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: "t".into(),
                },
                message: fixture_message_json("user", "grep it"),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: "t".into(),
                },
                message: json!({
                    "role": "assistant",
                    "content": [{
                        "type": "toolCall",
                        "id": "fc_grep1",
                        "name": "grep",
                        "arguments": { "pattern": "foo", "path": "src" }
                    }],
                    "timestamp": 0u64,
                }),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "tr1".into(),
                    parent_id: Some("a1".into()),
                    timestamp: "t".into(),
                },
                message: json!({
                    "role": "toolResult",
                    "toolCallId": "fc_grep1",
                    "toolName": "grep",
                    "content": [{ "type": "text", "text": "src/a.rs:1:foo" }],
                    "isError": false,
                    "timestamp": 0u64,
                }),
            }),
        ];
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: "tr1".into(),
            leaf_id: Some("tr1".into()),
            editor_text: None,
        };
        let mut ui = UiModel::default();
        rebuild_scrollback_from_travel(&mut ui, &entries, &travel);
        let tools: Vec<_> = ui
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::Tool { .. }))
            .collect();
        assert_eq!(tools.len(), 1, "got entries: {:?}", ui.entries);
        assert!(
            matches!(
                &tools[0],
                UiEntry::Tool {
                    id,
                    name,
                    args_preview,
                    output,
                    done: true,
                    is_error: false,
                    ..
                } if id == "fc_grep1"
                    && name == "grep"
                    && !args_preview.is_empty()
                    && output.contains("src/a.rs:1:foo")
            ),
            "got: {:?}",
            tools[0]
        );
    }

    #[test]
    fn rebuild_orphan_tool_result_is_single_done_tool() {
        let entries = vec![SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "tr_orphan".into(),
                parent_id: None,
                timestamp: "t".into(),
            },
            message: json!({
                "role": "toolResult",
                "toolCallId": "fc_missing",
                "toolName": "bash",
                "content": [{ "type": "text", "text": "hello" }],
                "isError": false,
                "timestamp": 0u64,
            }),
        })];
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: "tr_orphan".into(),
            leaf_id: Some("tr_orphan".into()),
            editor_text: None,
        };
        let mut ui = UiModel::default();
        rebuild_scrollback_from_travel(&mut ui, &entries, &travel);
        assert!(
            matches!(
                ui.entries.as_slice(),
                [UiEntry::Tool {
                    id,
                    name,
                    output,
                    done: true,
                    ..
                }] if id == "fc_missing" && name == "bash" && output.contains("hello")
            ),
            "got: {:?}",
            ui.entries
        );
    }
}
