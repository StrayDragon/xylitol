//! Tool args preview, path chrome, and quiet success output helpers.

use serde_json::Value;

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

/// Prefer `~/…` when path is under $HOME (pi `shortenPath` chrome).
fn shorten_tool_path(path: &str) -> String {
    let Ok(home) = std::env::var("HOME") else {
        return path.to_string();
    };
    if home.is_empty() {
        return path.to_string();
    }
    if path == home {
        return "~/".into();
    }
    let prefix = format!("{home}/");
    if let Some(rest) = path.strip_prefix(&prefix) {
        return format!("~/{rest}");
    }
    path.to_string()
}

/// Human-readable collapsed tool args (c1260 / c1280 / c1300).
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
            .map(|p| format!("read {}", shorten_tool_path(&p)))
            .unwrap_or_else(|| "read".into()),
        "ls" => pick_str(&["path", "dir"])
            .map(|p| format!("ls {}", shorten_tool_path(&p)))
            .unwrap_or_else(|| "ls".into()),
        "edit" => pick_str(&["path", "file"])
            .or_else(path_from_edits)
            .map(|p| format!("edit {}", shorten_tool_path(&p)))
            .unwrap_or_else(|| "edit".into()),
        "write" => {
            let path = pick_str(&["path", "file"]).map(|p| shorten_tool_path(&p));
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
            .map(|p| shorten_tool_path(&p))
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

/// Pull `display_diff` from edit-tool JSON result (shape is intentionally fragile).
pub fn extract_display_diff(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    value
        .get("display_diff")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

// Used to be for Diff summary; kept for edit-tool result shape.
#[allow(dead_code)]
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
    use crate::app::core::driver::XyEvent;
    use crate::app::tui::bridge::{UiEntry, UiModel, apply_xy_event};

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
        let tool = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id,
                output,
                display_diff,
                ..
            } if id == "e1" => Some((output.as_str(), display_diff.as_deref())),
            _ => None,
        });
        assert_eq!(tool, Some(("", Some("--- a\n+++ b\n"))));
        assert!(
            model
                .entries
                .iter()
                .all(|e| !matches!(e, UiEntry::Diff { .. })),
            "expected single Tool block, got {:?}",
            model.entries
        );
    }

    #[test]
    fn write_intent_streams_content_body() {
        use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};

        let mut model = UiModel::new();
        model.begin_run("hi");
        let partial = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::ToolCall {
                id: "w1".into(),
                name: "write".into(),
                arguments: serde_json::json!({
                    "path": "a.py",
                    "content": "def f():\n    return 1\n"
                }),
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
        let body = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id, write_content, ..
            } if id == "w1" => write_content.as_deref(),
            _ => None,
        });
        assert_eq!(body, Some("def f():\n    return 1\n"));
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
        assert!(
            model.entries.iter().any(|e| matches!(
                e,
                UiEntry::Tool {
                    id,
                    display_diff: Some(d),
                    ..
                } if id == "e1" && d.contains("fn main")
            )),
            "expected edit display_diff on Tool, got {:?}",
            model.entries
        );
        assert!(
            model
                .entries
                .iter()
                .all(|e| !matches!(e, UiEntry::Diff { .. })),
            "must not push a second Diff block"
        );
    }
}
