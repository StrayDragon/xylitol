//! Rebuild live scrollback after MessageHistory travel (c615 / c646).

use crate::app::tool_display::{is_ask_tool, is_write_tool};
use crate::protocol::session::{
    SessionEntry, SessionTreeTravel, TodoList, is_env_custom_message, is_tool_call_part,
    latest_agent_todo, message_parts, message_role, message_text, tool_call_name,
};
use crate::utils::elapsed_from_persist_ms;
use serde_json::Value;

use super::{
    BashBlockStatus, CompactionBlockStatus, UiEntry, UiModel, UiPhase, allocate_thinking_id,
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
    ui_model.streaming_think_id = None;
    ui_model.thought_clock.reset();
    ui_model.current_role = None;

    let path = ancestry_path_ids(entries, travel.leaf_id.as_deref());
    // c2760: bash lifecycle fold — done ids win; a lone running row (crash /
    // interrupted) renders as interrupted instead of a pending block.
    let bash_done_ids = bash_done_ids(entries);
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
            if role == Some("bashExecution")
                && m.message.get("status").and_then(Value::as_str) == Some("running")
            {
                let bash_id = m
                    .message
                    .get("bashId")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if bash_done_ids.contains(bash_id) {
                    continue; // the done row renders the block
                }
                let mut msg = m.message.clone();
                msg["output"] =
                    Value::String("[interrupted: process ended before completion]".into());
                msg["cancelled"] = Value::Bool(true);
                msg["status"] = Value::String("done".into());
                for ui in nested_bash_to_ui(&msg, &ui_model.entries) {
                    ui_model.entries.push(ui);
                }
                continue;
            }
        }
        let thought_elapsed = persisted_thinking_elapsed(entry);
        for ui in session_entry_to_ui_entries_with_thought_elapsed(
            entry,
            thought_elapsed,
            &ui_model.entries,
        ) {
            ui_model.entries.push(ui);
        }
    }
    sync_todo_checklist_from_entries(ui_model, entries);
}

/// Upsert the single latest-wins Todo checklist row from a full leaf branch (atd8/atd9).
pub fn sync_todo_checklist_from_entries(ui_model: &mut UiModel, entries: &[SessionEntry]) {
    sync_todo_checklist(ui_model, latest_agent_todo(entries).unwrap_or_default());
}

/// Latest-wins 待办栏 list (tool End or resume). Empty table occupies 0 dock rows.
pub fn sync_todo_checklist(ui_model: &mut UiModel, list: TodoList) {
    ui_model.todo = list;
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

    if is_ask_tool(name) {
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
            timeout_secs: None,
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

pub(crate) fn ancestry_path_ids(entries: &[SessionEntry], leaf_id: Option<&str>) -> Vec<String> {
    crate::protocol::session::transcript_ancestry_ids(entries, leaf_id)
}

/// Truncate opaque ids for the travel banner (full UUID path overflows COLS and
/// can wedge differential render / CapturedScreen in PTY E2E).
fn short_entry_id(id: &str) -> &str {
    const KEEP: usize = 8;
    if id.len() > KEEP { &id[..KEEP] } else { id }
}

/// Collect `bash_id`s that have a finished (`done`) row (c2760).
fn bash_done_ids(entries: &[SessionEntry]) -> std::collections::HashSet<String> {
    entries
        .iter()
        .filter_map(|e| {
            let SessionEntry::Message(m) = e else {
                return None;
            };
            if message_role(&m.message) != Some("bashExecution") {
                return None;
            }
            let status = m
                .message
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("done");
            if status != "done" {
                return None;
            }
            m.message
                .get("bashId")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

/// Project one session entry into zero or more UI rows (c646: thinking ≠ text).
///
/// `prior` carries the rows projected before this entry so Thinking ids count
/// ordinals globally, matching the live flush (att21).
#[cfg(test)]
fn session_entry_to_ui_entries(entry: &SessionEntry) -> Vec<UiEntry> {
    session_entry_to_ui_entries_with_thought_elapsed(entry, None, &[])
}

fn session_entry_to_ui_entries_with_thought_elapsed(
    entry: &SessionEntry,
    thought_elapsed: Option<u64>,
    prior: &[UiEntry],
) -> Vec<UiEntry> {
    match entry {
        SessionEntry::Message(m) if message_role(&m.message) == Some("bashExecution") => {
            nested_bash_to_ui(&m.message, prior)
        }
        // c1905: Env CustomMessage (session_env) must not appear as chat / ScrollNotice.
        SessionEntry::Message(m) if is_env_custom_message(&m.message) => Vec::new(),
        SessionEntry::Message(m) => {
            message_json_to_ui_entries(&m.base.id, &m.message, thought_elapsed, prior)
        }
        SessionEntry::CustomMessage(_) => Vec::new(),
        SessionEntry::Compaction(c) => vec![UiEntry::Compaction {
            status: CompactionBlockStatus::Complete,
            summary: c.summary.clone(),
            tokens_before: c.tokens_before,
            detail: None,
            tokens_after: None,
        }],
        SessionEntry::BranchSummary(b) => vec![UiEntry::ScrollNotice {
            text: format!("[branch] {}", b.summary),
        }],
        _ => Vec::new(),
    }
}

fn persisted_thinking_elapsed(entry: &SessionEntry) -> Option<u64> {
    match entry {
        SessionEntry::Message(m) => {
            let timing = m.message.get("streamTiming");
            elapsed_from_persist_ms(
                m.message
                    .get("thinkingElapsedSecs")
                    .and_then(|v| v.as_u64()),
                timing
                    .and_then(|t| t.get("thinkingStartedAtMs"))
                    .and_then(|v| v.as_u64()),
                timing
                    .and_then(|t| t.get("thinkingEndedAtMs"))
                    .and_then(|v| v.as_u64()),
            )
        }
        _ => None,
    }
}

fn nested_bash_to_ui(message: &Value, prior: &[UiEntry]) -> Vec<UiEntry> {
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
    // att30 fold key: prefer persisted bashId; fall back to command hash +
    // prior Bash ordinal so identical commands keep distinct fold keys.
    let id = message
        .get("bashId")
        .or_else(|| message.get("bash_id"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| UiModel::bash_block_id_from_command(prior, &command));
    let status = if cancelled {
        BashBlockStatus::Cancelled
    } else if exit_code.is_some_and(|c| c != 0) {
        BashBlockStatus::Error
    } else {
        BashBlockStatus::Success
    };
    vec![UiEntry::Bash {
        id,
        command,
        status,
        output,
        exclude_from_context,
    }]
}

fn message_json_to_ui_entries(
    entry_id: &str,
    message: &Value,
    thought_elapsed: Option<u64>,
    prior: &[UiEntry],
) -> Vec<UiEntry> {
    let Some(role) = message_role(message) else {
        return Vec::new();
    };
    // Belt-and-suspenders: custom env rows never become User/ScrollNotice.
    if is_env_custom_message(message) {
        return Vec::new();
    }
    match role {
        "user" => vec![UiEntry::User {
            text: message_text(message),
        }],
        "assistant" => assistant_parts_to_ui(entry_id, message, thought_elapsed, prior),
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
                timeout_secs: None,
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

fn assistant_parts_to_ui(
    entry_id: &str,
    message: &Value,
    thought_elapsed: Option<u64>,
    prior: &[UiEntry],
) -> Vec<UiEntry> {
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
                    let text = t.to_string();
                    let id = allocate_thinking_id(prior.iter().chain(out.iter()), &text);
                    out.push(UiEntry::Thinking {
                        id,
                        text,
                        elapsed_secs: thought_elapsed.filter(|s| *s > 0),
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
                if is_ask_tool(name) {
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
                    .cloned()
                    .unwrap_or(Value::Object(Default::default()));
                let mcp = is_mcp_tool_name(name);
                out.push(UiEntry::Tool {
                    timeout_secs: None,
                    id,
                    name: name.to_string(),
                    args_preview: if mcp {
                        String::new()
                    } else {
                        crate::app::tui::bridge::human_tool_args_preview(name, &args, usize::MAX)
                    },
                    tool_path: crate::app::tui::bridge::extract_tool_path(&args),
                    write_content: is_write_tool(name)
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
    use crate::app::core::driver::XyEvent;
    use crate::app::tui::bridge::apply_xy_event;
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
                    timestamp: 0,
                },
                message: fixture_message_json("user", "hi"),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: 0,
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
                timestamp: 0,
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
                UiEntry::Thinking { text, elapsed_secs: None, .. } ,
                UiEntry::Assistant { text: reply }
            ] if text == "step 1" && reply == "hello"),
            "got: {ui:?}"
        );
    }

    /// att21: live 与 rebuild 对同一逻辑块 MUST 同 id。直播 flush 的序数按
    /// 全量 entries 计；rebuild 对跨消息重复的同文 thinking 不得重置序数。
    #[test]
    fn rebuild_thinking_ids_match_live_ordinals_across_messages() {
        let mk_assistant = |id: &str, parent: &str| {
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: id.into(),
                    parent_id: Some(parent.into()),
                    timestamp: 0,
                },
                message: json!({
                    "role": "assistant",
                    "content": [
                        { "type": "thinking", "thinking": "same plan" },
                        { "type": "text", "text": "reply" }
                    ],
                    "timestamp": 0u64,
                }),
            })
        };
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": "hi" }],
                    "timestamp": 0u64,
                }),
            }),
            mk_assistant("a1", "u1"),
            mk_assistant("a2", "a1"),
        ];
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: "a2".into(),
            leaf_id: Some("a2".into()),
            editor_text: None,
        };
        // 直播口径：同文 thinking 第二次 flush 时序数为 1。
        let mut live = UiModel::default();
        apply_xy_event(&mut live, &XyEvent::ThinkingDelta("same plan".into()));
        apply_xy_event(&mut live, &XyEvent::TextDelta("reply".into()));
        apply_xy_event(&mut live, &XyEvent::ThinkingDelta("same plan".into()));
        apply_xy_event(&mut live, &XyEvent::TextDelta("reply".into()));
        let live_ids: Vec<_> = live
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::Thinking { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(live_ids.len(), 2);

        let mut ui = UiModel::default();
        rebuild_scrollback_from_travel(&mut ui, &entries, &travel);
        let rebuilt_ids: Vec<_> = ui
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::Thinking { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            rebuilt_ids, live_ids,
            "att21: rebuild ids must match live flush ordinals for identical thinking text"
        );
    }

    #[test]
    fn rebuild_omits_elapsed_without_thinking_stamps() {
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": "hi" }],
                    "timestamp": 1_700_000_000_000u64,
                }),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: 0,
                },
                message: json!({
                    "role": "assistant",
                    "content": [
                        { "type": "thinking", "thinking": "step 1" },
                        { "type": "text", "text": "hello" }
                    ],
                    "timestamp": 1_700_000_017_000u64,
                }),
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
            matches!(
                ui.entries.as_slice(),
                [
                    UiEntry::User { .. },
                    UiEntry::Thinking { elapsed_secs: None, text, .. },
                    UiEntry::Assistant { .. }
                ] if text == "step 1"
            ),
            "resume MUST omit Thought duration without thinkingElapsedSecs or streamTiming: {:?}",
            ui.entries
        );
    }

    #[test]
    fn rebuild_prefers_persisted_thinking_elapsed_over_adjacent_stamps() {
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": "hi" }],
                    "timestamp": 1_700_000_000_000u64,
                }),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: 0,
                },
                message: json!({
                    "role": "assistant",
                    "content": [
                        { "type": "thinking", "thinking": "step 1" },
                        { "type": "text", "text": "hello" }
                    ],
                    "timestamp": 1_700_000_017_000u64,
                    "thinkingElapsedSecs": 4u64,
                }),
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
            matches!(
                ui.entries.as_slice(),
                [
                    UiEntry::User { .. },
                    UiEntry::Thinking {
                        elapsed_secs: Some(4),
                        ..
                    },
                    UiEntry::Assistant { .. }
                ]
            ),
            "persisted thinkingElapsedSecs MUST win over adjacent message stamps: {:?}",
            ui.entries
        );
    }

    #[test]
    fn rebuild_computes_elapsed_from_thinking_node_ms() {
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": "hi" }],
                    "timestamp": 1_700_000_000_000u64,
                }),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: 0,
                },
                message: json!({
                    "role": "assistant",
                    "content": [
                        { "type": "thinking", "thinking": "step 1" },
                        { "type": "text", "text": "hello" }
                    ],
                    "timestamp": 1_700_000_017_000u64,
                    "streamTiming": {
                        "thinkingStartedAtMs": 1_700_000_010_000u64,
                        "thinkingEndedAtMs": 1_700_000_012_500u64,
                    },
                }),
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
            matches!(
                ui.entries.as_slice(),
                [
                    UiEntry::User { .. },
                    UiEntry::Thinking {
                        elapsed_secs: Some(2),
                        ..
                    },
                    UiEntry::Assistant { .. }
                ]
            ),
            "resume MUST subtract streamTiming thinkingEndedAtMs - thinkingStartedAtMs: {:?}",
            ui.entries
        );
    }

    #[test]
    fn compaction_entry_maps_to_collapsed_block() {
        let entry = SessionEntry::Compaction(crate::protocol::session::CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: Some("u1".into()),
                timestamp: 0,
            },
            summary: "## Goal\nkeep going".into(),
            first_kept_entry_id: "u2".into(),
            tokens_before: 186_842,
            details: None,
            from_hook: None,
            policy: None,
        });
        let ui = session_entry_to_ui_entries(&entry);
        assert!(
            matches!(
                ui.as_slice(),
                [UiEntry::Compaction {
                    status: CompactionBlockStatus::Complete,
                    tokens_before: 186_842,
                    tokens_after: None,
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
                timestamp: 0,
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
                timestamp: 0,
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
    fn session_env_custom_message_emits_no_ui_rows() {
        let entry = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "env1".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: json!({
                "role": "custom",
                "customType": "session_env",
                "content": "<session_env>\n  <date>2026-08-10</date>\n  <cwd>/tmp</cwd>\n</session_env>",
                "display": "session_env",
                "details": { "date": "2026-08-10", "cwd": "/tmp", "clock": "2026-08-10T00:00:00Z" },
            }),
        });
        let ui = session_entry_to_ui_entries(&entry);
        assert!(
            ui.is_empty(),
            "session_env must not surface in scrollback: {ui:?}"
        );
    }

    /// att36 / att50: the todo_* block (header preview + glyph-checklist body)
    /// and the checklist projection row must be identical after a live flush
    /// and after a travel rebuild from the same session content.
    #[test]
    fn rebuild_todo_block_matches_live_flush() {
        let todo_args = json!({ "items": [
            { "id": "a", "content": "检查环境", "status": "in_progress" },
            { "id": "b", "content": "写清单" }
        ]});
        let todo_result = json!({ "items": [
            { "id": "a", "content": "检查环境", "status": "in_progress" },
            { "id": "b", "content": "写清单", "status": "pending" }
        ]})
        .to_string();
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: fixture_message_json("user", "plan it"),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: 0,
                },
                message: json!({
                    "role": "assistant",
                    "content": [{
                        "type": "toolCall",
                        "id": "tc-todo",
                        "name": "todo_rewrite",
                        "arguments": todo_args,
                    }],
                    "timestamp": 0u64,
                }),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "tr1".into(),
                    parent_id: Some("a1".into()),
                    timestamp: 0,
                },
                message: json!({
                    "role": "toolResult",
                    "toolCallId": "tc-todo",
                    "toolName": "todo_rewrite",
                    "content": [{ "type": "text", "text": todo_result }],
                    "isError": false,
                    "timestamp": 0u64,
                }),
            }),
            // SSOT snapshot (atd2): the resume-side checklist source.
            SessionEntry::Custom(crate::protocol::session::CustomEntry {
                base: EntryBase {
                    entry_type: "custom".into(),
                    id: "snap1".into(),
                    parent_id: Some("tr1".into()),
                    timestamp: 0,
                },
                custom_type: crate::protocol::session::CUSTOM_TYPE_AGENT_TODO.into(),
                data: json!({ "items": [
                    { "id": "a", "content": "检查环境", "status": "in_progress" },
                    { "id": "b", "content": "写清单", "status": "pending" }
                ]}),
            }),
        ];
        let leaf = crate::protocol::session::transcript_leaf_anchor(&entries, None);
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: leaf.clone().unwrap_or_default(),
            leaf_id: leaf,
            editor_text: None,
        };
        let mut rebuilt = UiModel::default();
        rebuild_scrollback_from_travel(&mut rebuilt, &entries, &travel);

        // Same turn as a live event stream: user row → Start → TodoUpdated
        // (atd13) → End.
        let mut live = UiModel::default();
        live.begin_run("plan it");
        apply_xy_event(
            &mut live,
            &XyEvent::ToolExecutionStart {
                id: "tc-todo".into(),
                name: "todo_rewrite".into(),
                args: json!({ "items": [
                    { "id": "a", "content": "检查环境", "status": "in_progress" },
                    { "id": "b", "content": "写清单" }
                ]}),
            },
        );
        apply_xy_event(
            &mut live,
            &XyEvent::TodoUpdated {
                list: TodoList::new(vec![
                    crate::protocol::session::TodoItem {
                        id: "a".into(),
                        content: "检查环境".into(),
                        status: crate::protocol::session::TodoStatus::InProgress,
                    },
                    crate::protocol::session::TodoItem {
                        id: "b".into(),
                        content: "写清单".into(),
                        status: crate::protocol::session::TodoStatus::Pending,
                    },
                ]),
            },
        );
        apply_xy_event(
            &mut live,
            &XyEvent::ToolExecutionEnd {
                id: "tc-todo".into(),
                name: "todo_rewrite".into(),
                result: json!({ "items": [
                    { "id": "a", "content": "检查环境", "status": "in_progress" },
                    { "id": "b", "content": "写清单", "status": "pending" }
                ]})
                .to_string(),
                is_error: false,
            },
        );

        let projection = |model: &UiModel| -> Vec<String> {
            model
                .entries
                .iter()
                .map(|e| match e {
                    UiEntry::Tool {
                        args_preview,
                        output,
                        done,
                        is_error,
                        ..
                    } => format!(
                        "tool[preview={args_preview:?} body={output:?} done={done} err={is_error}]"
                    ),
                    UiEntry::User { text } => format!("user[{text}]"),
                    other => format!("other[{other:?}]"),
                })
                .collect()
        };
        assert_eq!(
            projection(&rebuilt),
            projection(&live),
            "att36: rebuild must be shape-identical to the live flush"
        );
        assert_eq!(
            projection(&live),
            vec![
                "user[plan it]".to_string(),
                "tool[preview=\"2 items · 1 in progress\" body=\"[~] 检查环境\\n[ ] 写清单\" done=true err=false]".to_string(),
            ],
            "att13/att36: humanized preview + checklist body on the tool row"
        );
        assert_eq!(
            rebuilt.todo, live.todo,
            "待办栏 list must match live/resume"
        );
        assert_eq!(live.todo.items.len(), 2);
    }

    #[test]
    fn rebuild_merges_tool_call_and_result_into_one_tool() {
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: fixture_message_json("user", "grep it"),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: 0,
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
                    timestamp: 0,
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
                timestamp: 0,
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

    #[test]
    fn rebuild_splices_parentless_bookkeeping_seam_in_chain() {
        // Historical pollution: messages chained THROUGH a parent-less
        // modelChange. Resume rebuild must show the full history, not just the
        // rows after the seam.
        let entries = vec![
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u1".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                message: fixture_message_json("user", "first"),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a1".into(),
                    parent_id: Some("u1".into()),
                    timestamp: 0,
                },
                message: fixture_message_json("assistant", "reply one"),
            }),
            SessionEntry::ModelChange(crate::protocol::session::ModelChangeEntry {
                base: EntryBase {
                    entry_type: "model_change".into(),
                    id: "mc_seam".into(),
                    parent_id: None,
                    timestamp: 0,
                },
                provider: "fake".into(),
                model_id: "fake/m".into(),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "u2".into(),
                    parent_id: Some("mc_seam".into()),
                    timestamp: 0,
                },
                message: fixture_message_json("user", "second"),
            }),
            SessionEntry::Message(MessageEntry {
                base: EntryBase {
                    entry_type: "message".into(),
                    id: "a2".into(),
                    parent_id: Some("u2".into()),
                    timestamp: 0,
                },
                message: fixture_message_json("assistant", "reply two"),
            }),
        ];
        let leaf = crate::protocol::session::transcript_leaf_anchor(&entries, None);
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: leaf.clone().unwrap_or_default(),
            leaf_id: leaf,
            editor_text: None,
        };
        let mut ui = UiModel::default();
        rebuild_scrollback_from_travel(&mut ui, &entries, &travel);
        let users: Vec<_> = ui
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::User { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            users,
            vec!["first", "second"],
            "seam splice must keep pre-seam user rows, got {users:?}"
        );
    }
}

#[cfg(test)]
mod bash_lifecycle_fold_tests {
    use super::*;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};
    use serde_json::json;

    fn bash_row(base_id: &str, bash_id: &str, status: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: base_id.into(),
                parent_id: None,
                timestamp: 0,
            },
            message: json!({
                "role": "bashExecution",
                "bashId": bash_id,
                "command": "echo hi",
                "output": if status == "running" { "" } else { "hi\n" },
                "cancelled": false,
                "truncated": false,
                "excludeFromContext": false,
                "status": status,
            }),
        })
    }

    fn rebuild(entries: &[SessionEntry]) -> UiModel {
        let mut model = UiModel::default();
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: entries
                .last()
                .and_then(|e| e.entry_id())
                .unwrap_or("")
                .to_string(),
            leaf_id: entries
                .last()
                .and_then(|e| e.entry_id())
                .map(str::to_string),
            editor_text: None,
        };
        rebuild_scrollback_from_travel(&mut model, entries, &travel);
        model
    }

    #[test]
    fn running_with_done_renders_only_done() {
        let entries = vec![
            bash_row("b-start", "b1", "running"),
            bash_row("b-done", "b1", "done"),
        ];
        let model = rebuild(&entries);
        let bash_blocks: Vec<_> = model
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::Bash { .. }))
            .collect();
        assert_eq!(bash_blocks.len(), 1, "start row must fold into done");
    }

    #[test]
    fn lone_running_renders_interrupted() {
        let entries = vec![bash_row("b-start", "b1", "running")];
        let model = rebuild(&entries);
        let output = model
            .entries
            .iter()
            .find_map(|e| match e {
                UiEntry::Bash { output, .. } => Some(output.clone()),
                _ => None,
            })
            .expect("bash block");
        assert!(
            output.contains("interrupted"),
            "expected interrupted marker: {output:?}"
        );
    }

    #[test]
    fn legacy_row_without_status_renders_done() {
        let entries = vec![SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "old".into(),
                parent_id: None,
                timestamp: 0,
            },
            message: json!({
                "role": "bashExecution",
                "command": "ls",
                "output": "a",
                "cancelled": false,
                "truncated": false,
                "excludeFromContext": false,
            }),
        })];
        let model = rebuild(&entries);
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Bash { .. })),
            "legacy row must remain a normal bash block"
        );
    }
}
