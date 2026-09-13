//! Tool args preview, path display, and quiet success output helpers.

use std::path::{Path, PathBuf};

use serde_json::Value;
use xylitol_tui::{ASK_HEADER_A_MAX, ASK_HEADER_Q_MAX, ellipsize_ask_frag};

use crate::protocol::session::{TodoList, TodoStatus};

/// Shared todo status glyph — checklist projection row and todo_* block body
/// MUST use this one table (att36).
pub(crate) fn todo_status_glyph(status: TodoStatus) -> &'static str {
    match status {
        TodoStatus::Pending => "[ ]",
        TodoStatus::InProgress => "[~]",
        TodoStatus::Completed => "[x]",
        TodoStatus::Cancelled => "[-]",
    }
}

/// Checklist body lines (status glyph + content), one per item.
pub(crate) fn todo_list_body_lines(list: &TodoList) -> Vec<String> {
    list.items
        .iter()
        .map(|i| format!("{} {}", todo_status_glyph(i.status), i.content))
        .collect()
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

/// Display path for tool output: under process cwd → relative; otherwise absolute.
///
/// Aligns with VS Code / CLI path disclosure (not `~/` shortening).
pub(crate) fn display_fs_path(path: &str) -> String {
    if path.is_empty() {
        return path.to_string();
    }
    let cwd = std::env::current_dir().ok();
    let input = Path::new(path);
    let abs: PathBuf = if input.is_absolute() {
        input.to_path_buf()
    } else if let Some(ref cwd) = cwd {
        cwd.join(input)
    } else {
        return path.replace('\\', "/");
    };

    if let Some(cwd) = cwd.as_ref()
        && let Ok(rel) = abs.strip_prefix(cwd)
    {
        let s = rel.to_string_lossy().replace('\\', "/");
        if !s.is_empty() {
            return s;
        }
    }
    abs.to_string_lossy().replace('\\', "/")
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
        Some(p) => display_fs_path(p),
        None => PATH_PLACEHOLDER.to_string(),
    }
}

/// VS Code–style location suffix: `:line`, `:line:col`, or `:start-end`.
fn format_read_line_range(args: &Value) -> String {
    let offset = args.get("offset").and_then(json_u64);
    let limit = args.get("limit").and_then(json_u64);
    let col = args
        .get("column")
        .or_else(|| args.get("col"))
        .and_then(json_u64);
    if offset.is_none() && limit.is_none() {
        return String::new();
    }
    let start = offset.unwrap_or(1);
    match (limit, col) {
        (Some(lim), _) => {
            let end = start.saturating_add(lim).saturating_sub(1);
            if end == start {
                format!(":{start}")
            } else {
                format!(":{start}-{end}")
            }
        }
        (None, Some(c)) => format!(":{start}:{c}"),
        (None, None) => format!(":{start}"),
    }
}

/// Title-case tool name for scrollback display (`read` → `Read`).
pub(crate) fn display_tool_title(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

/// Human-readable collapsed tool **location** (no tool-name prefix; name painted separately).
///
/// Shape: `<path>[:line[:col]|:start-end]` or `$ <cmd>` for bash.
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

    let resolved_path = path_override
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .or_else(|| extract_tool_path(args));

    let summary = match name {
        "bash" | "shell" => pick_str(&["command", "cmd"])
            .map(|c| format!("$ {c}"))
            .unwrap_or_else(|| PATH_PLACEHOLDER.to_string()),
        "read" => {
            format!(
                "{}{}",
                path_slot(resolved_path.as_deref()),
                format_read_line_range(args)
            )
        }
        "ls" | "edit" | "write" => path_slot(resolved_path.as_deref()),
        "find" => pick_str(&["pattern", "path", "glob"])
            .map(|p| display_fs_path(&p))
            .unwrap_or_else(|| PATH_PLACEHOLDER.to_string()),
        "grep" => pick_str(&["pattern", "path"])
            .map(|p| {
                if p.contains('/') || Path::new(&p).extension().is_some() {
                    display_fs_path(&p)
                } else {
                    p
                }
            })
            .unwrap_or_else(|| PATH_PLACEHOLDER.to_string()),
        "todo_list" | "todo_rewrite" | "todo_update" => {
            // att13: item-count summary, never the full items JSON. No parsed
            // items yet (streaming / read-only todo_list) → pathless placeholder.
            match args.get("items").and_then(Value::as_array) {
                None => PATH_PLACEHOLDER.to_string(),
                Some(items) => {
                    let in_progress = items
                        .iter()
                        .filter(|i| {
                            i.get("status").and_then(Value::as_str) == Some("in_progress")
                        })
                        .count();
                    let n = items.len();
                    let mut summary = format!("{n} item{}", if n == 1 { "" } else { "s" });
                    if in_progress > 0 {
                        summary.push_str(&format!(" · {in_progress} in progress"));
                    }
                    summary
                }
            }
        }
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
        .map(|p| display_fs_path(&p))
        .unwrap_or_default(),
    };

    compact_json_preview(&Value::String(summary), max_chars)
}

pub fn human_tool_args_preview(name: &str, args: &Value, max_chars: usize) -> String {
    human_tool_args_preview_with_path(name, args, None, max_chars)
}

/// Map built-in tool `result` → TUI body text (no machine JSON remnants).
///
/// Returns `None` when the result should stay as-is (errors, unknown shapes, plain text).
pub(crate) fn humanize_tool_result_for_tui(
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
        "read" => humanize_read_tool_output(result),
        "bash" | "shell" => humanize_bash_tool_output(result),
        "todo_list" | "todo_rewrite" | "todo_update" => {
            // att36: block body is the checklist (shared glyph table), not the
            // raw items JSON. Empty list → empty body (same as write/edit).
            serde_json::from_str::<Value>(result)
                .ok()
                .and_then(|v| TodoList::from_data_value(&v).ok())
                .map(|list| todo_list_body_lines(&list).join("\n"))
        }
        "ask" => {
            let (phase, summary, _) = humanize_ask_result(result, false);
            let _ = phase;
            Some(summary)
        }
        _ => None,
    }
}

/// Parse ask-tool JSON into scrollback phase + human lines (no raw JSON primary).
///
/// Header (`summary`) keeps a compact `Ask · q → a · …` shape with ellipsized
/// question ids / answers; expandable `detail_lines` stay full.
pub(crate) fn humanize_ask_result(
    result: &str,
    is_error: bool,
) -> (crate::app::tui::bridge::AskPhase, String, Vec<String>) {
    use crate::app::tui::bridge::AskPhase;
    if is_error {
        return (
            AskPhase::Skipped,
            "Ask · 失败".into(),
            vec![result.trim().to_string()],
        );
    }
    let Ok(value) = serde_json::from_str::<Value>(result.trim()) else {
        return (AskPhase::Answered, "Ask · 已答".into(), vec![]);
    };
    let status = value.get("status").and_then(Value::as_str).unwrap_or("");
    match status {
        "skipped" => (
            AskPhase::Skipped,
            "Ask · 已跳过 · 按已有信息继续".into(),
            vec!["（用户跳过本题）".into()],
        ),
        "answered" => {
            let answers = value
                .get("answers")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let detail_lines: Vec<String> = answers
                .iter()
                .map(|a| {
                    let id = a.get("id").and_then(Value::as_str).unwrap_or("?");
                    let labels: Vec<&str> = a
                        .get("labels")
                        .and_then(Value::as_array)
                        .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                        .unwrap_or_default();
                    if labels.is_empty() {
                        format!("{id} → （空）")
                    } else {
                        format!("{id} → {}", labels.join(", "))
                    }
                })
                .collect();
            let summary = if answers.is_empty() {
                "Ask · 已答".into()
            } else {
                let parts: Vec<String> = answers
                    .iter()
                    .map(|a| {
                        let id = a.get("id").and_then(Value::as_str).unwrap_or("?");
                        let labels: Vec<&str> = a
                            .get("labels")
                            .and_then(Value::as_array)
                            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
                            .unwrap_or_default();
                        let q = ellipsize_ask_frag(id, ASK_HEADER_Q_MAX);
                        if labels.is_empty() {
                            format!("{q} → （空）")
                        } else {
                            let a = ellipsize_ask_frag(&labels.join(", "), ASK_HEADER_A_MAX);
                            format!("{q} → {a}")
                        }
                    })
                    .collect();
                format!("Ask · {}", parts.join(" · "))
            };
            (AskPhase::Answered, summary, detail_lines)
        }
        _ => (AskPhase::Answered, "Ask · 已答".into(), vec![]),
    }
}

/// True when `text` is a JSON object that still looks like tool wire remnants.
pub(crate) fn output_looks_like_machine_json(text: &str) -> bool {
    let trimmed = text.trim();
    if !(trimmed.starts_with('{') && trimmed.ends_with('}')) {
        return false;
    }
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return false;
    };
    let Some(obj) = value.as_object() else {
        return false;
    };
    const MACHINE_KEYS: &[&str] = &[
        "success",
        "bytes",
        "display_diff",
        "diff",
        "total_lines",
        "exit_code",
        "combined",
        "stdout",
        "stderr",
        "full_output_path",
        "truncated_by",
        "remaining_lines",
    ];
    // read-shaped: content + total_lines (or offset-only envelope)
    if obj.contains_key("content")
        && (obj.contains_key("total_lines")
            || obj.contains_key("offset")
            || obj.contains_key("truncated"))
    {
        return true;
    }
    // todo-shaped: full-snapshot envelope (att36 safety net for orphans).
    if obj.contains_key("items")
        && obj.get("items").is_some_and(Value::is_array)
        && obj.len() == 1
    {
        return true;
    }
    MACHINE_KEYS.iter().any(|k| obj.contains_key(*k))
}

fn humanize_read_tool_output(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    let obj = value.as_object()?;
    // Read success envelope always has `content` (may be empty).
    if !obj.contains_key("content") {
        return None;
    }
    let content = obj.get("content").and_then(Value::as_str).unwrap_or("");
    let mut body = content.to_string();
    if obj
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        if let Some(hint) = obj.get("hint").and_then(Value::as_str) {
            if !body.is_empty() && !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(hint);
        } else if let Some(rem) = obj.get("remaining_lines").and_then(Value::as_u64) {
            if !body.is_empty() && !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(&format!("(truncated — {rem} lines remaining)"));
        }
    }
    if let Some(note) = obj.get("note").and_then(Value::as_str) {
        // e.g. offset exceeds file length
        if body.is_empty() {
            return Some(note.to_string());
        }
        if !body.ends_with('\n') {
            body.push('\n');
        }
        body.push_str(note);
    }
    Some(body)
}

/// Turn bash/shell tool JSON into shell-like stdout/stderr text (not raw JSON remnants).
pub(crate) fn humanize_bash_tool_output(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    if !value.is_object() {
        return None;
    }
    // Require at least one bash-shaped key so we don't swallow unknown tools.
    let has_bash_shape = value.get("exit_code").is_some()
        || value.get("combined").is_some()
        || value.get("stdout").is_some();
    if !has_bash_shape {
        return None;
    }

    let combined = value
        .get("combined")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            value
                .get("stdout")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("");
    let stderr = value
        .get("stderr")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    let exit_code = value
        .get("exit_code")
        .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|n| n as i64)));
    let truncated = value
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let full_path = value
        .get("full_output_path")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());

    let mut body = String::new();
    if !combined.is_empty() {
        body.push_str(combined.trim_end());
    }
    if !stderr.is_empty() {
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(stderr.trim_end());
    }
    if truncated {
        if !body.is_empty() {
            body.push('\n');
        }
        if let Some(path) = full_path {
            body.push_str(&format!(
                "[Full output: {path}. Truncated: (see file) lines shown]"
            ));
        } else {
            body.push_str("(truncated)");
        }
    }
    if let Some(code) = exit_code.filter(|&c| c != 0) {
        if !body.is_empty() {
            body.push('\n');
        }
        body.push_str(&format!("(exit {code})"));
    }
    if body.is_empty() {
        return Some("(no output)".into());
    }
    Some(body)
}

/// Line-range suffix from edit `display_diff` (`:start-end` on the new side).
pub(crate) fn extract_line_range_from_display_diff(diff: &str) -> Option<String> {
    let mut min_new = u32::MAX;
    let mut max_new = 0u32;
    let mut found = false;

    for line in diff.lines() {
        let hunk = line
            .trim()
            .strip_prefix("...")
            .map(str::trim)
            .and_then(|s| s.strip_prefix('|'))
            .map(str::trim)
            .unwrap_or(line.trim());
        if let Some(hunk) = hunk.strip_prefix("@@") {
            if let Some((start, count)) = parse_unified_new_span(hunk) {
                found = true;
                min_new = min_new.min(start);
                let end = if count == 0 {
                    start
                } else {
                    start.saturating_add(count).saturating_sub(1)
                };
                max_new = max_new.max(end.max(start));
            }
            continue;
        }
        // display_diff rows: "     NNNN | …" (new-only) or "OOOO NNNN | …"
        if let Some((a, b)) = parse_display_diff_line_nos(line) {
            found = true;
            if let Some(n) = b.or(a) {
                min_new = min_new.min(n);
                max_new = max_new.max(n);
            }
        }
    }

    if !found || min_new == u32::MAX {
        return None;
    }
    if min_new == max_new {
        Some(format!(":{min_new}"))
    } else {
        Some(format!(":{min_new}-{max_new}"))
    }
}

fn parse_unified_new_span(hunk_after_at: &str) -> Option<(u32, u32)> {
    // " -12,5 +14,7 @@ …" or " -12 +14 @@"
    let rest = hunk_after_at.trim();
    let mut parts = rest.split_whitespace();
    let _old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    let mut nums = new.split(',');
    let start: u32 = nums.next()?.parse().ok()?;
    let count: u32 = nums.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    Some((start, count))
}

fn parse_display_diff_line_nos(line: &str) -> Option<(Option<u32>, Option<u32>)> {
    let (gutter, _) = line.split_once('|')?;
    let gutter = gutter.trim_end();
    if gutter.contains("...") {
        return None;
    }
    let mut nums = gutter
        .split_whitespace()
        .filter_map(|t| t.parse::<u32>().ok());
    let a = nums.next();
    let b = nums.next();
    if a.is_none() && b.is_none() {
        return None;
    }
    Some((a, b))
}

/// Append `:range` to a path-only preview if missing.
pub(crate) fn merge_path_preview_with_range(preview: &str, range: &str) -> String {
    if preview.is_empty() || range.is_empty() || preview.starts_with('$') {
        return preview.to_string();
    }
    for (i, ch) in preview.char_indices().rev() {
        if ch == ':' {
            let suffix = &preview[i..];
            if suffix.len() > 1 && suffix.as_bytes()[1].is_ascii_digit() {
                return preview.to_string();
            }
        }
    }
    format!("{preview}{range}")
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

/// True when header still has no real path slot (`...` or empty).
pub(crate) fn preview_lacks_real_path(_name: &str, preview: &str) -> bool {
    let rest = preview.trim();
    rest.is_empty() || rest == PATH_PLACEHOLDER || rest.starts_with("...")
}

/// Prefer keeping a richer streaming header over a weaker later snapshot.
pub(crate) fn preview_is_downgrade(name: &str, old: &str, new: &str) -> bool {
    if old.is_empty() || old == new {
        return false;
    }
    if new == name || new == PATH_PLACEHOLDER {
        return true;
    }
    match name {
        "bash" | "shell" => new == name || new == "$ ..." || new == PATH_PLACEHOLDER,
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
    fn write_summary_is_path_only() {
        let preview = human_tool_args_preview(
            "write",
            &serde_json::json!({
                "path": "docs/a.md",
                "content": "line1\nline2\nline3"
            }),
            80,
        );
        assert_eq!(preview, "docs/a.md");
        assert!(!preview.contains("line1"), "got {preview}");
        assert!(!preview.contains('{'));
    }

    #[test]
    fn write_content_only_uses_path_ellipsis() {
        let preview =
            human_tool_args_preview("write", &serde_json::json!({"content": "a\nb\nc"}), 80);
        assert_eq!(preview, "...");
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
        assert_eq!(preview, "src/main.rs");
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
        assert_eq!(preview, "...");
    }

    #[test]
    fn file_path_alias_is_recognized() {
        let preview =
            human_tool_args_preview("edit", &serde_json::json!({"file_path": "lib.rs"}), 80);
        assert_eq!(preview, "lib.rs");
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
        assert_eq!(preview, "README.md:120-329");
    }

    #[test]
    fn read_range_offset_only() {
        let preview = human_tool_args_preview(
            "read",
            &serde_json::json!({"path": "a.rs", "offset": 5}),
            80,
        );
        assert_eq!(preview, "a.rs:5");
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
        use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

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
        assert_eq!(preview, Some("a.py"));
    }

    #[test]
    fn edit_path_streams_from_partial_args() {
        use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

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
        assert_eq!(preview, Some("/tmp/x"));
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
        use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

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
        assert_eq!(after, Some("a.py"));
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
        assert_eq!(before.as_deref(), Some("..."));
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
        assert_eq!(after, Some("src/a.rs:1"));
        assert!(!after.unwrap().contains("oldText"));
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
        use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

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
        assert_eq!(preview, Some("src/a.rs:1"));
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
        let truncated =
            "tail-line\n[Full output: /tmp/x.log. Truncated: 1 lines shown (50.0KB limit)]"
                .to_string();
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

    #[test]
    fn humanize_bash_json_to_shell_text() {
        let out = humanize_bash_tool_output(
            r#"{"stdout":"","stderr":"","exit_code":0,"combined":"","truncated":false}"#,
        );
        assert_eq!(out.as_deref(), Some("(no output)"));

        let out = humanize_bash_tool_output(
            r#"{"stdout":"hi\n","stderr":"","exit_code":0,"combined":"hi\n","truncated":false}"#,
        );
        assert_eq!(out.as_deref(), Some("hi"));

        let out = humanize_bash_tool_output(
            r#"{"stdout":"","stderr":"boom","exit_code":2,"combined":"","truncated":false}"#,
        );
        assert_eq!(out.as_deref(), Some("boom\n(exit 2)"));
    }

    #[test]
    fn humanize_read_shows_content_not_json() {
        let out = humanize_tool_result_for_tui(
            "read",
            r#"{"content":"hello\nworld\n","total_lines":2}"#,
            false,
        );
        assert_eq!(out.as_deref(), Some("hello\nworld\n"));
        assert!(!output_looks_like_machine_json(out.as_deref().unwrap()));
    }

    #[test]
    fn humanize_read_keeps_truncation_hint() {
        let out = humanize_tool_result_for_tui(
            "read",
            r#"{"content":"line1\n","total_lines":99,"truncated":true,"hint":"Output truncated. 90 lines remaining."}"#,
            false,
        )
        .unwrap();
        assert!(out.starts_with("line1\n"));
        assert!(out.contains("Output truncated"));
        assert!(!out.contains("\"total_lines\""));
    }

    #[test]
    fn builtin_tool_results_never_leak_machine_json_in_tui() {
        // Typical success envelopes from infra tools → TUI body must not be raw JSON remnants.
        let cases: &[(&str, &str, &str)] = &[
            ("write", r#"{"success":true,"path":"a.py","bytes":12}"#, ""),
            (
                "edit",
                r#"{"success":true,"path":"a.py","display_diff":"1 1 | x","diff":"---"}"#,
                "",
            ),
            (
                "read",
                r#"{"content":"hello\nworld\n","total_lines":2}"#,
                "hello\nworld\n",
            ),
            (
                "bash",
                r#"{"stdout":"","stderr":"","exit_code":0,"combined":"","truncated":false}"#,
                "(no output)",
            ),
            (
                "bash",
                r#"{"stdout":"ok\n","stderr":"","exit_code":0,"combined":"ok\n","truncated":false}"#,
                "ok",
            ),
        ];
        for (name, result, expect) in cases {
            let human = humanize_tool_result_for_tui(name, result, false)
                .unwrap_or_else(|| panic!("{name} should humanize: {result}"));
            assert_eq!(human, *expect, "humanize({name})");
            assert!(
                !output_looks_like_machine_json(&human),
                "{name} still looks like machine JSON: {human}"
            );
            assert!(!human.contains("\"success\""), "{name} leaked success key");
            assert!(
                !human.contains("\"total_lines\""),
                "{name} leaked total_lines"
            );
            assert!(!human.contains("\"exit_code\""), "{name} leaked exit_code");
            assert!(
                !human.contains("\"display_diff\""),
                "{name} leaked display_diff"
            );
            assert!(!human.contains("\"bytes\""), "{name} leaked bytes");
        }
    }

    #[test]
    fn todo_args_preview_is_item_count_not_json() {
        // att13: count summary; in_progress suffix only when non-zero.
        let preview = human_tool_args_preview(
            "todo_rewrite",
            &serde_json::json!({"items": [
                {"content": "a", "status": "in_progress"},
                {"content": "b"}
            ]}),
            80,
        );
        assert_eq!(preview, "2 items · 1 in progress");
        let solo = human_tool_args_preview(
            "todo_rewrite",
            &serde_json::json!({"items": [{"content": "a"}]}),
            80,
        );
        assert_eq!(solo, "1 item");
        // No parsed items yet (todo_list / partial stream) → pathless placeholder.
        assert_eq!(
            human_tool_args_preview("todo_list", &serde_json::json!({}), 80),
            "..."
        );
    }

    #[test]
    fn humanize_todo_result_is_glyph_checklist() {
        // att36: block body is the checklist via the shared glyph table.
        let out = humanize_tool_result_for_tui(
            "todo_rewrite",
            r#"{"items":[{"id":"a","content":"one","status":"in_progress"},{"id":"b","content":"two","status":"completed"}]}"#,
            false,
        )
        .expect("todo result humanizes");
        assert_eq!(out, "[~] one\n[x] two");
        assert!(!output_looks_like_machine_json(&out));

        // Cleared list → empty body (same quiet-success family as write/edit).
        assert_eq!(
            humanize_tool_result_for_tui("todo_update", r#"{"items":[]}"#, false).as_deref(),
            Some("")
        );
        // Errors keep raw text (early return), never the checklist.
        assert_eq!(
            humanize_tool_result_for_tui("todo_rewrite", "unknown todo id: x", true),
            None
        );
    }

    #[test]
    fn machine_json_recognizes_todo_snapshot_envelope() {
        assert!(output_looks_like_machine_json(r#"{"items":[]}"#));
        assert!(output_looks_like_machine_json(
            r#"{"items":[{"id":"a","content":"x","status":"pending"}]}"#
        ));
        // Mixed-shape objects (e.g. MCP payloads that happen to have `items`)
        // stay unrecognized — only the exact single-key snapshot envelope.
        assert!(!output_looks_like_machine_json(r#"{"items":[1],"total":1}"#));
    }

    #[test]
    fn read_tool_end_applies_content_to_scrollback_output() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "r1".into(),
                name: "read".into(),
                args: serde_json::json!({"path": "/tmp/note.md"}),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "r1".into(),
                name: "read".into(),
                result: r#"{"content":"hello\nworld\n","total_lines":2}"#.into(),
                is_error: false,
            },
        );
        let out = model.entries.iter().find_map(|e| match e {
            UiEntry::Tool { id, output, .. } if id == "r1" => Some(output.as_str()),
            _ => None,
        });
        assert_eq!(out, Some("hello\nworld\n"));
        assert!(!output_looks_like_machine_json(out.unwrap()));
    }

    #[test]
    fn ls_grep_find_plain_text_results_pass_through() {
        // These tools already return human text — humanize returns None; body stays plain.
        for name in ["ls", "grep", "find"] {
            assert!(
                humanize_tool_result_for_tui(name, "src/a.rs\nsrc/b.rs\n", false).is_none(),
                "{name} plain text should not be rewritten"
            );
            assert!(!output_looks_like_machine_json("src/a.rs\nsrc/b.rs\n"));
        }
    }

    #[test]
    fn ask_header_ellipsizes_long_q_and_a_body_full() {
        let long_a = "📊 看诊断, 📝 查符号, 🔍 搜代码, 🚀 跑命令, 更多选项";
        let json = format!(
            r#"{{"status":"answered","answers":[{{"id":"demo_multi","labels":[{}]}}]}}"#,
            long_a
                .split(", ")
                .map(|s| format!(r#""{s}""#))
                .collect::<Vec<_>>()
                .join(",")
        );
        let (phase, summary, detail) = humanize_ask_result(&json, false);
        assert_eq!(phase, crate::app::tui::bridge::AskPhase::Answered);
        assert!(summary.starts_with("Ask · "), "{summary}");
        assert!(summary.contains('…') || summary.contains('→'), "{summary}");
        assert!(
            !summary.contains("更多选项"),
            "header must truncate long answers: {summary}"
        );
        assert_eq!(detail.len(), 1);
        assert!(
            detail[0].contains("更多选项"),
            "body must keep full labels: {}",
            detail[0]
        );
    }
}
