//! XyEvent → UI-only model seam (c465 / c494).
//!
//! Render / `UiRoot` MUST consume [`UiModel`] only — never match [`XyEvent`].
//! Event-family logic lives in per-family handler modules; [`apply_xy_event`] remains the sole entry.

mod handlers;
mod model;
pub(crate) mod session_tree;

pub(crate) use model::trailing_aborted_note;
pub use model::{BashBlockStatus, QueueBadge, UiEntry, UiModel, UiPhase};

use serde_json::Value;

use crate::app::core::driver::XyEvent;
use crate::domain::message::AgentMessage;

/// Single seam: translate one [`XyEvent`] into UI-only mutations.
///
/// Unhandled / metadata variants are logged and ignored — never panic.
/// Family handlers live in per-family modules; this remains the only public entry.
pub fn apply_xy_event(model: &mut UiModel, event: &XyEvent) {
    if handlers::apply_agent_family(model, event)
        || handlers::apply_stream_family(model, event)
        || handlers::apply_tools_family(model, event)
        || handlers::apply_lifecycle_family(model, event)
    {
        return;
    }
    log::debug!(target: "xylitol::tui", "XyEvent unhandled by bridge event={}", event.description());
}

pub(crate) fn compact_json_preview(value: &Value, max_chars: usize) -> String {
    let raw = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if raw.chars().count() <= max_chars {
        return raw;
    }
    let truncated: String = raw.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{truncated}…")
}

/// Human-readable collapsed tool args (c1260 / c1280).
///
/// Missing key fields → short placeholder. Never dump full args JSON as chrome.
pub(crate) fn human_tool_args_preview(name: &str, args: &Value, max_chars: usize) -> String {
    let pick_str = |keys: &[&str]| -> Option<String> {
        for key in keys {
            if let Some(s) = args
                .get(*key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                return Some(s.to_string());
            }
        }
        None
    };

    let path_from_edits = || -> Option<String> {
        args.get("edits")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|edit| {
                ["path", "file"]
                    .iter()
                    .find_map(|k| edit.get(*k).and_then(Value::as_str))
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            })
    };

    let content_line_count = || -> Option<usize> {
        let content = args.get("content").and_then(Value::as_str)?;
        if content.is_empty() {
            return None;
        }
        Some(content.lines().count().max(1))
    };

    let summary = match name {
        "bash" | "shell" => pick_str(&["command", "cmd"])
            .map(|c| format!("$ {c}"))
            .unwrap_or_else(|| name.to_string()),
        "read" => pick_str(&["path", "file"])
            .map(|p| format!("read {p}"))
            .unwrap_or_else(|| "read".into()),
        "ls" => pick_str(&["path", "dir"])
            .map(|p| format!("ls {p}"))
            .unwrap_or_else(|| "ls".into()),
        "edit" => pick_str(&["path", "file"])
            .or_else(path_from_edits)
            .map(|p| format!("edit {p}"))
            .unwrap_or_else(|| "edit".into()),
        "write" => {
            let path = pick_str(&["path", "file"]);
            match (path, content_line_count()) {
                (Some(p), Some(n)) => format!("write {p} ({n} lines)"),
                (Some(p), None) => format!("write {p}"),
                (None, Some(n)) => format!("write ({n} lines)"),
                (None, None) => "write".into(),
            }
        }
        "find" => pick_str(&["pattern", "path", "glob"])
            .map(|p| format!("find {p}"))
            .unwrap_or_else(|| "find".into()),
        "grep" => pick_str(&["pattern", "path"])
            .map(|p| format!("grep {p}"))
            .unwrap_or_else(|| "grep".into()),
        _ => pick_str(&["path", "file", "command", "cmd", "pattern", "query", "url"])
            .unwrap_or_default(),
    };

    compact_json_preview(&Value::String(summary), max_chars)
}

/// Success write/edit machine JSON is not default chrome (c1280); errors stay visible.
pub(crate) fn quiet_tool_success_output(
    name: &str,
    result: &str,
    is_error: bool,
) -> Option<String> {
    if is_error {
        return None;
    }
    match name {
        "write" | "edit" => {
            let trimmed = result.trim_start();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                Some(String::new())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Upsert a pending tool row from streaming intent (MessageUpdate) or execution start.
pub(crate) fn upsert_tool_entry(model: &mut UiModel, id: &str, name: &str, args: &Value) {
    let preview = human_tool_args_preview(name, args, 80);
    if let Some(UiEntry::Tool {
        name: n,
        args_preview,
        ..
    }) = find_tool_mut(&mut model.entries, id)
    {
        *n = name.to_string();
        *args_preview = preview;
        return;
    }
    model.entries.push(UiEntry::Tool {
        id: id.to_string(),
        name: name.to_string(),
        args_preview: preview,
        output: String::new(),
        is_error: false,
        done: false,
    });
}

/// Sync ToolCall parts from a partial assistant message (c1255 → c1260).
///
/// Flushes any in-flight thinking/text **before** mounting tools so scrollback
/// order matches provider order (ThinkingDelta* → ToolCall*), not
/// tool-row-then-late-flush-thinking.
pub(crate) fn sync_tool_intent_from_message(model: &mut UiModel, message: &AgentMessage) {
    use crate::domain::message::{AgentPart, LlmMessage};

    let AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. }) = message else {
        return;
    };
    let has_tool = content
        .iter()
        .any(|p| matches!(p, AgentPart::ToolCall { .. }));
    if !has_tool {
        return;
    }
    model.flush_streaming();
    for part in content {
        if let AgentPart::ToolCall {
            id,
            name,
            arguments,
        } = part
        {
            upsert_tool_entry(model, id, name, arguments);
        }
    }
}

pub(crate) fn find_tool_mut<'a>(entries: &'a mut [UiEntry], id: &str) -> Option<&'a mut UiEntry> {
    entries.iter_mut().rev().find(|e| match e {
        UiEntry::Tool { id: tid, .. } => tid == id,
        _ => false,
    })
}

/// Append a user scrollback row; skip if it duplicates the trailing user entry
/// (e.g. idle `begin_run` already seeded the same prompt).
pub(crate) fn push_user_entry_dedup(model: &mut UiModel, text: String) {
    if let Some(UiEntry::User { text: last }) = model.entries.last()
        && last == &text
    {
        return;
    }
    model.entries.push(UiEntry::User { text });
}

/// Pull `display_diff` from edit-tool JSON result (shape is intentionally fragile).
pub fn extract_display_diff(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    value
        .get("display_diff")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

pub(crate) fn extract_edit_path(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    value
        .get("path")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::AgentMessage;

    fn assistant_end() -> XyEvent {
        XyEvent::AgentEnd {
            messages: Vec::<AgentMessage>::new(),
        }
    }

    #[test]
    fn text_deltas_commit_on_message_end() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
        );
        apply_xy_event(&mut model, &XyEvent::TextDelta("Hello".into()));
        apply_xy_event(&mut model, &XyEvent::TextDelta("!".into()));
        apply_xy_event(
            &mut model,
            &XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        );
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Assistant { text } if text == "Hello!"))
        );
        assert_eq!(model.streaming_assistant, "");
    }

    #[test]
    fn turn_end_does_not_idle() {
        let mut model = UiModel::new();
        model.begin_run("x");
        apply_xy_event(&mut model, &XyEvent::TurnStart { turn_index: 0 });
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "1".into(),
                name: "read".into(),
                args: Value::Null,
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "1".into(),
                name: "read".into(),
                result: "ok".into(),
                is_error: false,
            },
        );
        apply_xy_event(&mut model, &XyEvent::TurnEnd { turn_index: 0 });
        assert_eq!(model.phase, UiPhase::Busy);
        assert!(model.status.is_some());
    }

    #[test]
    fn agent_end_idles_when_follow_up_empty() {
        let mut model = UiModel::new();
        model.begin_run("x");
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 0,
                follow_up_count: 0,
            },
        );
        apply_xy_event(&mut model, &assistant_end());
        assert_eq!(model.phase, UiPhase::Idle);
        assert_eq!(model.status, None);
    }

    #[test]
    fn agent_end_stays_busy_with_follow_up() {
        let mut model = UiModel::new();
        model.begin_run("x");
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 0,
                follow_up_count: 1,
            },
        );
        apply_xy_event(&mut model, &assistant_end());
        assert_eq!(model.phase, UiPhase::Busy);
        assert_eq!(model.queue.follow_up_count, 1);
    }

    #[test]
    fn queue_update_sets_badge() {
        let mut model = UiModel::new();
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 2,
                follow_up_count: 3,
            },
        );
        assert_eq!(
            model.queue,
            QueueBadge {
                steer_count: 2,
                follow_up_count: 3
            }
        );
    }

    #[test]
    fn queue_update_trims_pending_strip_fifo() {
        let mut model = UiModel::new();
        model.enqueue_steer_strip("a".into());
        model.enqueue_steer_strip("b".into());
        model.enqueue_follow_up_strip("c".into());
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 1,
                follow_up_count: 0,
            },
        );
        assert_eq!(model.pending_steer, vec!["b".to_string()]);
        assert!(model.pending_follow_up.is_empty());
    }

    #[test]
    fn user_message_start_commits_steer_to_scrollback() {
        use crate::domain::message::AgentMessage;

        let mut model = UiModel::new();
        model.begin_run("hello");
        model.enqueue_steer_strip("nudge".into());
        apply_xy_event(
            &mut model,
            &XyEvent::MessageStart {
                role: "user".into(),
                message: Some(AgentMessage::user("nudge")),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::MessageEnd {
                role: "user".into(),
                message: Some(AgentMessage::user("nudge")),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 0,
                follow_up_count: 0,
            },
        );
        assert!(model.pending_steer.is_empty());
        let users: Vec<_> = model
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::User { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(users, ["hello", "nudge"]);
    }

    #[test]
    fn aborted_error_is_system_note_and_idles() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(&mut model, &XyEvent::Error("aborted".into()));
        assert_eq!(model.phase, UiPhase::Idle);
        assert!(model.status.is_none());
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "Aborted"))
        );
        assert!(
            !model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Error { .. }))
        );
    }

    #[test]
    fn note_bash_cancelled_does_not_emit_aborted() {
        let mut model = UiModel::new();
        model.begin_bash_block("sleep 1", false);
        model.note_bash_cancelled();
        assert!(model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Bash {
                status: BashBlockStatus::Cancelled,
                output,
                ..
            } if output.contains("(cancelled)")
        )));
        assert!(
            !model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "Aborted"))
        );
    }

    #[test]
    fn note_user_abort_allows_second_abort_after_new_command() {
        let mut model = UiModel::new();
        model.note_user_abort();
        model.entries.push(UiEntry::System {
            text: "$ sleep 2".into(),
        });
        model.note_user_abort();
        let n = model
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::System { text } if text == "Aborted"))
            .count();
        assert_eq!(
            n, 2,
            "each bang abort must show Aborted: {:?}",
            model.entries
        );
    }

    #[test]
    fn note_user_abort_dedupes_with_error_aborted() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        model.note_user_abort();
        apply_xy_event(&mut model, &XyEvent::Error("aborted".into()));
        let n = model
            .entries
            .iter()
            .filter(
                |e| matches!(e, UiEntry::System { text } if text == "Aborted" || text == "aborted"),
            )
            .count();
        assert_eq!(n, 1, "must not duplicate Aborted notes");
        assert_eq!(model.phase, UiPhase::Idle);
    }

    #[test]
    fn edit_tool_end_extracts_display_diff() {
        let mut model = UiModel::new();
        model.begin_run("edit please");
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "e1".into(),
                name: "edit".into(),
                args: Value::Null,
            },
        );
        let result = r#"{"success":true,"path":"src/a.rs","display_diff":"1 1 | fn main() {}","diff":"---"}"#;
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "e1".into(),
                name: "edit".into(),
                result: result.into(),
                is_error: false,
            },
        );
        assert!(model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Diff {
                summary,
                display_diff
            } if summary.contains("src/a.rs") && display_diff.contains("fn main")
        )));
    }

    #[test]
    fn metadata_events_do_not_panic() {
        let mut model = UiModel::new();
        apply_xy_event(
            &mut model,
            &XyEvent::ModelSelect {
                provider: "openai".into(),
                model_id: "gpt".into(),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ThinkingLevelChanged {
                level: "high".into(),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::SessionInfoChanged {
                key: "k".into(),
                value: Value::Null,
            },
        );
        assert!(model.entries.is_empty());
        assert_eq!(model.phase, UiPhase::Idle);
    }

    #[test]
    fn message_update_does_not_duplicate_deltas() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(&mut model, &XyEvent::TextDelta("Hello".into()));
        apply_xy_event(
            &mut model,
            &XyEvent::MessageUpdate {
                text: "Hello".into(),
                thinking: None,
                message: None,
            },
        );
        apply_xy_event(&mut model, &XyEvent::TextDelta("!".into()));
        apply_xy_event(
            &mut model,
            &XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        );
        let assistants: Vec<_> = model
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::Assistant { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(assistants, ["Hello!"]);
    }

    #[test]
    fn bash_summary_is_human_readable() {
        let preview =
            human_tool_args_preview("bash", &serde_json::json!({"command": "ls -la"}), 80);
        assert!(preview.starts_with("$ ls"), "got {preview}");
        assert!(!preview.contains('{'));
    }

    #[test]
    fn write_summary_includes_line_count_not_content() {
        let preview = human_tool_args_preview(
            "write",
            &serde_json::json!({
                "path": "docs/a.md",
                "content": "line1\nline2\nline3"
            }),
            80,
        );
        assert!(preview.contains("docs/a.md"), "got {preview}");
        assert!(preview.contains("3 lines"), "got {preview}");
        assert!(!preview.contains("line1"), "got {preview}");
        assert!(!preview.contains('{'));
    }

    #[test]
    fn edit_summary_from_edits_path_not_json_wall() {
        let preview = human_tool_args_preview(
            "edit",
            &serde_json::json!({
                "edits": [{
                    "path": "src/main.rs",
                    "oldText": "fn a() {}",
                    "newText": "fn a() { todo!() }"
                }]
            }),
            80,
        );
        assert_eq!(preview, "edit src/main.rs");
        assert!(!preview.contains("oldText"));
        assert!(!preview.contains('{'));
    }

    #[test]
    fn edit_without_path_is_short_placeholder() {
        let preview = human_tool_args_preview(
            "edit",
            &serde_json::json!({
                "edits": [{"oldText": "a", "newText": "b"}]
            }),
            80,
        );
        assert_eq!(preview, "edit");
    }

    #[test]
    fn unknown_tool_large_args_not_json_wall() {
        let preview = human_tool_args_preview(
            "custom_tool",
            &serde_json::json!({
                "payload": {"a": 1, "b": "x".repeat(200)},
                "meta": {"nested": true}
            }),
            80,
        );
        assert!(!preview.contains("payload"), "got {preview}");
        assert!(!preview.contains('{'), "got {preview}");
    }

    #[test]
    fn write_success_end_quiets_tool_output() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "w1".into(),
                name: "write".into(),
                args: serde_json::json!({"path": "a.md", "content": "hi\n"}),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "w1".into(),
                name: "write".into(),
                result: r#"{"path":"a.md","success":true}"#.into(),
                is_error: false,
            },
        );
        let output = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id, output, done, ..
            } if id == "w1" => Some((output.as_str(), *done)),
            _ => None,
        });
        assert_eq!(output, Some(("", true)));
    }

    #[test]
    fn edit_success_end_quiets_output_keeps_diff() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "e1".into(),
                name: "edit".into(),
                args: serde_json::json!({"path": "a.rs"}),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "e1".into(),
                name: "edit".into(),
                result: r#"{"path":"a.rs","display_diff":"--- a\n+++ b\n","success":true}"#.into(),
                is_error: false,
            },
        );
        let tool_out = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool { id, output, .. } if id == "e1" => Some(output.as_str()),
            _ => None,
        });
        assert_eq!(tool_out, Some(""));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Diff { summary, .. } if summary.contains("a.rs"))),
            "expected Diff entry, got {:?}",
            model.entries
        );
    }

    #[test]
    fn edit_error_keeps_result_output() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "e2".into(),
                name: "edit".into(),
                args: serde_json::json!({"path": "a.rs"}),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "e2".into(),
                name: "edit".into(),
                result: "exact match failed".into(),
                is_error: true,
            },
        );
        let tool_out = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id,
                output,
                is_error,
                ..
            } if id == "e2" => Some((output.as_str(), *is_error)),
            _ => None,
        });
        assert_eq!(tool_out, Some(("exact match failed", true)));
    }

    #[test]
    fn message_update_tool_intent_flushes_thinking_first() {
        use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};

        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(&mut model, &XyEvent::ThinkingDelta("plan…".into()));
        assert!(
            model
                .entries
                .iter()
                .all(|e| !matches!(e, UiEntry::Thinking { .. })),
            "thinking still in streaming buffer"
        );

        let partial = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![
                AgentPart::thinking("plan…"),
                AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "find".into(),
                    arguments: serde_json::json!({"pattern": "**/*.md"}),
                },
            ],
            stop_reason: None,
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        });
        apply_xy_event(
            &mut model,
            &XyEvent::MessageUpdate {
                text: String::new(),
                thinking: Some("plan…".into()),
                message: Some(partial),
            },
        );

        let kinds: Vec<&str> = model
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::Thinking { .. } => Some("thinking"),
                UiEntry::Tool { name, .. } => Some(name.as_str()),
                UiEntry::Assistant { .. } => Some("assistant"),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds,
            ["thinking", "find"],
            "scrollback must be thinking then tool, got {kinds:?}"
        );
        assert!(model.streaming_thinking.is_empty());
    }

    #[test]
    fn message_update_creates_tool_before_execution() {
        use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};

        let mut model = UiModel::new();
        model.begin_run("hi");
        let partial = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::ToolCall {
                id: "call-1".into(),
                name: "bash".into(),
                arguments: serde_json::json!({"command": "echo hi"}),
            }],
            stop_reason: None,
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        });
        apply_xy_event(
            &mut model,
            &XyEvent::MessageUpdate {
                text: String::new(),
                thinking: None,
                message: Some(partial),
            },
        );
        let tools: Vec<_> = model
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::Tool {
                    id,
                    args_preview,
                    done,
                    ..
                } => Some((id.as_str(), args_preview.as_str(), *done)),
                _ => None,
            })
            .collect();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].0, "call-1");
        assert!(tools[0].1.starts_with("$ echo"), "got {}", tools[0].1);
        assert!(!tools[0].2);

        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "call-1".into(),
                name: "bash".into(),
                args: serde_json::json!({"command": "echo hi"}),
            },
        );
        let tool_count = model
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::Tool { .. }))
            .count();
        assert_eq!(
            tool_count, 1,
            "ToolExecutionStart must upsert, not duplicate"
        );
    }

    #[test]
    fn message_update_streams_args_preview() {
        use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};

        let mut model = UiModel::new();
        model.begin_run("hi");
        let mk = |args: serde_json::Value| {
            AgentMessage::Llm(LlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: args,
                }],
                stop_reason: None,
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            })
        };
        apply_xy_event(
            &mut model,
            &XyEvent::MessageUpdate {
                text: String::new(),
                thinking: None,
                message: Some(mk(serde_json::json!({"path": "/tm"}))),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::MessageUpdate {
                text: String::new(),
                thinking: None,
                message: Some(mk(serde_json::json!({"path": "/tmp/x.rs"}))),
            },
        );
        let preview = model
            .entries
            .iter()
            .find_map(|e| match e {
                UiEntry::Tool { args_preview, .. } => Some(args_preview.as_str()),
                _ => None,
            })
            .expect("tool row");
        assert_eq!(preview, "read /tmp/x.rs");
        assert_eq!(
            model
                .entries
                .iter()
                .filter(|e| matches!(e, UiEntry::Tool { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn compaction_start_sets_status_and_note() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionStart {
                reason: "auto: 90%".into(),
            },
        );
        assert_eq!(model.phase, UiPhase::Busy);
        assert_eq!(model.status.as_deref(), Some("Compacting"));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("auto: 90%")))
        );
    }

    #[test]
    fn compaction_end_restores_working() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionStart {
                reason: "manual".into(),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionEnd {
                result: Some("ok".into()),
                aborted: false,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Working"));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "compaction complete"))
        );
    }

    #[test]
    fn compaction_end_aborted_restores_working() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionStart {
                reason: "manual".into(),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionEnd {
                result: None,
                aborted: true,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Working"));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "compaction aborted"))
        );
    }

    #[test]
    fn auto_retry_start_sets_status() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryStart {
                attempt: 2,
                max_retries: 5,
                delay_ms: 100,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Retry 2/5"));
    }

    #[test]
    fn auto_retry_end_fail_notes_and_restores() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryStart {
                attempt: 1,
                max_retries: 3,
                delay_ms: 0,
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryEnd {
                success: false,
                attempt: 1,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Working"));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("retry failed")))
        );
    }

    #[test]
    fn auto_retry_end_success_restores_without_fail_note() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryStart {
                attempt: 1,
                max_retries: 3,
                delay_ms: 0,
            },
        );
        let before = model.entries.len();
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryEnd {
                success: true,
                attempt: 1,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Working"));
        assert_eq!(model.entries.len(), before);
        assert!(
            !model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("retry failed")))
        );
    }
}
