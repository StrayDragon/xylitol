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

/// Streaming / empty path placeholder (pi `renderToolPath` → `...`).
const PATH_PLACEHOLDER: &str = "...";

fn json_u64(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_i64().and_then(|i| u64::try_from(i).ok()))
}

/// Top-level or edits[0] path (`path` / `file_path` / `file`).
pub(crate) fn extract_tool_path(args: &Value) -> Option<String> {
    for key in ["path", "file_path", "file"] {
        if let Some(s) = args
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            return Some(s.to_string());
        }
    }
    args.get("edits")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(|edit| {
            ["path", "file_path", "file"]
                .iter()
                .find_map(|k| edit.get(*k).and_then(Value::as_str))
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
}

/// Path from write/edit success JSON (End backfill).
pub(crate) fn extract_result_path(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    extract_tool_path(&value)
}

fn path_slot(path: Option<&str>) -> String {
    match path.filter(|p| !p.is_empty()) {
        Some(p) => shorten_tool_path(p),
        None => PATH_PLACEHOLDER.to_string(),
    }
}

fn format_read_line_range(args: &Value) -> String {
    let offset = args.get("offset").and_then(json_u64);
    let limit = args.get("limit").and_then(json_u64);
    if offset.is_none() && limit.is_none() {
        return String::new();
    }
    let start = offset.unwrap_or(1);
    match limit {
        Some(lim) => {
            let end = start.saturating_add(lim).saturating_sub(1);
            format!(":{start}-{end}")
        }
        None => format!(":{start}"),
    }
}

/// Human-readable collapsed tool args (c1260 / c1280 / c1300 / c1320).
///
/// `path_override`: sticky path when args lack one (partial JSON). `None` = derive from args.
pub(crate) fn human_tool_args_preview_with_path(
    name: &str,
    args: &Value,
    path_override: Option<&str>,
    max_chars: usize,
) -> String {
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

    let content_line_count = || -> Option<usize> {
        let content = args.get("content").and_then(Value::as_str)?;
        if content.is_empty() {
            return None;
        }
        Some(content.lines().count().max(1))
    };

    let resolved_path = path_override
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .or_else(|| extract_tool_path(args));

    let summary = match name {
        "bash" | "shell" => pick_str(&["command", "cmd"])
            .map(|c| format!("$ {c}"))
            .unwrap_or_else(|| name.to_string()),
        "read" => {
            format!(
                "read {}{}",
                path_slot(resolved_path.as_deref()),
                format_read_line_range(args)
            )
        }
        "ls" => format!("ls {}", path_slot(resolved_path.as_deref())),
        "edit" => format!("edit {}", path_slot(resolved_path.as_deref())),
        "write" => {
            let path = path_slot(resolved_path.as_deref());
            match content_line_count() {
                Some(n) => format!("write {path} ({n} lines)"),
                None => format!("write {path}"),
            }
        }
        "find" => pick_str(&["pattern", "path", "glob"])
            .map(|p| format!("find {p}"))
            .unwrap_or_else(|| "find".into()),
        "grep" => pick_str(&["pattern", "path"])
            .map(|p| format!("grep {p}"))
            .unwrap_or_else(|| "grep".into()),
        _ => pick_str(&[
            "path",
            "file_path",
            "file",
            "command",
            "cmd",
            "pattern",
            "query",
            "url",
        ])
        .map(|p| shorten_tool_path(&p))
        .unwrap_or_default(),
    };

    compact_json_preview(&Value::String(summary), max_chars)
}

pub(crate) fn human_tool_args_preview(name: &str, args: &Value, max_chars: usize) -> String {
    human_tool_args_preview_with_path(name, args, None, max_chars)
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

/// Pull pi-shaped `[Full output: …]` line from bash tool JSON / plain result.
pub(crate) fn extract_full_output_notice(result: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<Value>(result) {
        for key in ["combined", "stdout", "output"] {
            if let Some(text) = value.get(key).and_then(|v| v.as_str())
                && let Some(line) = text.lines().find(|l| l.starts_with("[Full output:"))
            {
                return Some(line.to_string());
            }
        }
        return None;
    }
    result
        .lines()
        .find(|l| l.starts_with("[Full output:"))
        .map(str::to_string)
}

/// When tool JSON is hard-truncated, return the truncated display body (att16).
pub(crate) fn extract_truncated_tool_display(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    let truncated = value.get("truncated").and_then(|v| v.as_bool())?;
    if !truncated {
        return None;
    }
    for key in ["combined", "stdout", "output"] {
        if let Some(text) = value.get(key).and_then(|v| v.as_str())
            && !text.is_empty()
        {
            return Some(text.to_string());
        }
    }
    None
}

/// Pull `display_diff` from edit-tool JSON result (shape is intentionally fragile).
pub fn extract_display_diff(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    value
        .get("display_diff")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// True when header still has no real path slot (`edit ...`, bare `write`, etc.).
pub(crate) fn preview_lacks_real_path(name: &str, preview: &str) -> bool {
    let prefixes = match name {
        "write" => &["write "][..],
        "edit" => &["edit "][..],
        "read" => &["read "][..],
        "ls" => &["ls "][..],
        _ => return false,
    };
    let Some(rest) = prefixes.iter().find_map(|p| preview.strip_prefix(p)) else {
        return true;
    };
    rest.is_empty() || rest.starts_with("...")
}

/// Prefer keeping a richer streaming header over a weaker later snapshot.
pub(crate) fn preview_is_downgrade(name: &str, old: &str, new: &str) -> bool {
    if old.is_empty() || old == new {
        return false;
    }
    if new == name || new == format!("{name} ...") {
        return true;
    }
    match name {
        "bash" | "shell" => new == name || new == "$ ...",
        "write" | "edit" | "read" | "ls" => {
            preview_lacks_real_path(name, new) && !preview_lacks_real_path(name, old)
        }
        _ => false,
    }
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
    fn write_content_only_uses_path_ellipsis() {
        let preview =
            human_tool_args_preview("write", &serde_json::json!({"content": "a\nb\nc"}), 80);
        assert_eq!(preview, "write ... (3 lines)");
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
    fn edit_without_path_is_ellipsis_placeholder() {
        let preview = human_tool_args_preview(
            "edit",
            &serde_json::json!({
                "edits": [{"oldText": "a", "newText": "b"}]
            }),
            80,
        );
        assert_eq!(preview, "edit ...");
    }

    #[test]
    fn file_path_alias_is_recognized() {
        let preview =
            human_tool_args_preview("edit", &serde_json::json!({"file_path": "lib.rs"}), 80);
        assert_eq!(preview, "edit lib.rs");
    }

    #[test]
    fn read_range_offset_and_limit() {
        let preview = human_tool_args_preview(
            "read",
            &serde_json::json!({
                "path": "README.md",
                "offset": 120,
                "limit": 210
            }),
            80,
        );
        assert_eq!(preview, "read README.md:120-329");
    }

    #[test]
    fn read_range_offset_only() {
        let preview = human_tool_args_preview(
            "read",
            &serde_json::json!({"path": "a.rs", "offset": 5}),
            80,
        );
        assert_eq!(preview, "read a.rs:5");
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
    fn write_path_sticky_across_content_only_upsert() {
        use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};

        let mut model = UiModel::new();
        model.begin_run("hi");
        let mk = |args: serde_json::Value| {
            AgentMessage::Llm(LlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "w1".into(),
                    name: "write".into(),
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
                message: Some(mk(serde_json::json!({"path": "a.py"}))),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::MessageUpdate {
                text: String::new(),
                thinking: None,
                message: Some(mk(serde_json::json!({"content": "x\ny\n"}))),
            },
        );
        let preview = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id, args_preview, ..
            } if id == "w1" => Some(args_preview.as_str()),
            _ => None,
        });
        assert_eq!(preview, Some("write a.py (2 lines)"));
    }

    #[test]
    fn edit_path_streams_from_partial_args() {
        use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};

        let mut model = UiModel::new();
        model.begin_run("hi");
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::ToolCall {
                id: "e1".into(),
                name: "edit".into(),
                arguments: serde_json::json!({"path": "/tmp/x"}),
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
                message: Some(msg),
            },
        );
        let preview = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id, args_preview, ..
            } if id == "e1" => Some(args_preview.as_str()),
            _ => None,
        });
        assert_eq!(preview, Some("edit /tmp/x"));
    }

    #[test]
    fn end_preserves_bash_command_preview() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "b1".into(),
                name: "bash".into(),
                args: serde_json::json!({"command": "python3 a.py"}),
            },
        );
        assert_eq!(
            model.entries.iter().find_map(|e| match e {
                UiEntry::Tool {
                    id, args_preview, ..
                } if id == "b1" => Some(args_preview.as_str()),
                _ => None,
            }),
            Some("$ python3 a.py")
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "b1".into(),
                name: "bash".into(),
                result: "ok\n".into(),
                is_error: false,
            },
        );
        let after = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id,
                args_preview,
                done,
                ..
            } if id == "b1" => Some((args_preview.as_str(), *done)),
            _ => None,
        });
        assert_eq!(after, Some(("$ python3 a.py", true)));
    }

    #[test]
    fn end_preserves_streamed_write_path() {
        use crate::domain::message::{AgentMessage, AgentPart, LlmMessage};

        let mut model = UiModel::new();
        model.begin_run("hi");
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::ToolCall {
                id: "w1".into(),
                name: "write".into(),
                arguments: serde_json::json!({
                    "path": "a.py",
                    "content": "print(1)\n"
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
                message: Some(msg),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "w1".into(),
                name: "write".into(),
                result: r#"{"path":"a.py","success":true}"#.into(),
                is_error: false,
            },
        );
        let after = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id, args_preview, ..
            } if id == "w1" => Some(args_preview.as_str()),
            _ => None,
        });
        assert_eq!(after, Some("write a.py (1 lines)"));
    }

    #[test]
    fn end_backfills_path_into_preview() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "e1".into(),
                name: "edit".into(),
                args: serde_json::json!({"edits": [{"oldText": "a", "newText": "b"}]}),
            },
        );
        let before = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id, args_preview, ..
            } if id == "e1" => Some(args_preview.clone()),
            _ => None,
        });
        assert_eq!(before.as_deref(), Some("edit ..."));
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "e1".into(),
                name: "edit".into(),
                result: r#"{"path":"src/a.rs","display_diff":"     1 | -a\n     1 | +b\n","success":true}"#.into(),
                is_error: false,
            },
        );
        let after = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id, args_preview, ..
            } if id == "e1" => Some(args_preview.as_str()),
            _ => None,
        });
        assert_eq!(after, Some("edit src/a.rs"));
        assert!(!after.unwrap().contains(":1"));
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
        let preview = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool {
                id, args_preview, ..
            } if id == "e1" => Some(args_preview.as_str()),
            _ => None,
        });
        assert_eq!(preview, Some("edit src/a.rs"));
    }

    #[test]
    fn truncated_bash_end_replaces_streamed_buffer() {
        let mut model = UiModel::new();
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "b1".into(),
                name: "bash".into(),
                args: serde_json::json!({"command": "yes"}),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionUpdate {
                id: "b1".into(),
                output: "STREAMED-HUGE\n".repeat(100),
            },
        );
        let truncated = format!(
            "tail-line\n[Full output: /tmp/x.log. Truncated: 1 lines shown (50.0KB limit)]"
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "b1".into(),
                name: "bash".into(),
                result: serde_json::json!({
                    "stdout": truncated,
                    "stderr": "",
                    "combined": truncated,
                    "truncated": true,
                    "full_output_path": "/tmp/x.log",
                    "exit_code": 0,
                })
                .to_string(),
                is_error: false,
            },
        );
        let out = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool { id, output, .. } if id == "b1" => Some(output.as_str()),
            _ => None,
        });
        let out = out.expect("bash tool");
        assert!(
            out.contains("[Full output:"),
            "must keep Full output footer: {out}"
        );
        assert!(
            !out.contains("STREAMED-HUGE"),
            "must drop streamed full buffer: {out}"
        );
        assert!(out.contains("tail-line"));
    }
}
