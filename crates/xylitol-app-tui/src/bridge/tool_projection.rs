use serde_json::Value;

use super::humanize_ask_result;
use super::model::{AskPhase, UiEntry, UiModel};
use super::preview::{
    extract_display_diff, extract_full_output_notice, extract_line_range_from_display_diff,
    extract_result_path, extract_tool_path, extract_truncated_tool_display,
    human_tool_args_preview_with_path, humanize_tool_result_for_tui, merge_path_preview_with_range,
    output_looks_like_machine_json, preview_is_downgrade, preview_lacks_real_path,
};
use crate::tool_display::{is_edit_tool, is_write_tool};

pub(crate) fn upsert_tool_entry(model: &mut UiModel, id: &str, name: &str, args: &Value) {
    use crate::tool_display::{is_mcp_tool_name, mcp_tool_body};

    let fresh_path = extract_tool_path(args);
    let timeout_secs = matches!(name, "bash" | "grep" | "find")
        .then(|| args.get("timeout"))
        .flatten()
        .and_then(Value::as_i64)
        .filter(|n| *n > 0)
        .map(|n| n.min(xylitol_plugin_sdk::MAX_TOOL_TIMEOUT_SECS as i64) as u64);
    let write_content = is_write_tool(name)
        .then(|| {
            args.get("content")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .flatten()
        .filter(|s| !s.is_empty());
    let mcp = is_mcp_tool_name(name);
    let mcp_preview = mcp.then(String::new);
    let mcp_pending_body = mcp.then(|| mcp_tool_body(Some(args), None));
    if let Some(UiEntry::Tool {
        name: n,
        timeout_secs: ts,
        args_preview,
        tool_path,
        write_content: wc,
        output,
        done,
        ..
    }) = find_tool_mut(&mut model.entries, id)
    {
        *n = name.to_string();
        if let Some(p) = fresh_path {
            *tool_path = Some(p);
        }
        if ts.is_none() {
            *ts = timeout_secs;
        }
        let new_preview = mcp_preview.clone().unwrap_or_else(|| {
            human_tool_args_preview_with_path(name, args, tool_path.as_deref(), usize::MAX)
        });
        if !preview_is_downgrade(name, args_preview, &new_preview) {
            *args_preview = new_preview;
        }
        if write_content.is_some() {
            *wc = write_content;
        }
        if let Some(body) = mcp_pending_body
            && !*done
            && (output.is_empty() || output.starts_with("args:"))
        {
            *output = body;
        }
        return;
    }
    let tool_path = fresh_path;
    let preview = mcp_preview.unwrap_or_else(|| {
        human_tool_args_preview_with_path(name, args, tool_path.as_deref(), usize::MAX)
    });
    model.entries.push(UiEntry::Tool {
        id: id.to_string(),
        name: name.to_string(),
        args_preview: preview.clone(),
        args_full: preview,
        tool_path,
        timeout_secs,
        write_content,
        display_diff: None,
        output: mcp_pending_body.unwrap_or_default(),
        is_error: false,
        done: false,
    });
}

pub(crate) fn find_tool_mut<'a>(entries: &'a mut [UiEntry], id: &str) -> Option<&'a mut UiEntry> {
    entries.iter_mut().rev().find(|e| match e {
        UiEntry::Tool { id: tid, .. } => tid == id,
        _ => false,
    })
}

fn ensure_visible_failure_line(output: &mut String, result: &str, is_error: bool, is_mcp: bool) {
    if !is_error || is_mcp || output.contains(result) {
        return;
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(result);
}

fn refresh_header_from_result_path(
    name: &str,
    result: &str,
    args_preview: &mut String,
    tool_path: &mut Option<String>,
    write_content: &Option<String>,
) {
    let Some(path) = extract_result_path(result) else {
        return;
    };
    let weak_preview = preview_lacks_real_path(name, args_preview);
    if tool_path.as_deref().filter(|p| !p.is_empty()).is_none() {
        *tool_path = Some(path);
    }
    if weak_preview {
        let mut synthetic = serde_json::Map::new();
        if let Some(p) = tool_path.as_deref() {
            synthetic.insert("path".into(), Value::String(p.to_string()));
        }
        if let Some(c) = write_content.as_deref() {
            synthetic.insert("content".into(), Value::String(c.to_string()));
        }
        *args_preview = human_tool_args_preview_with_path(
            name,
            &Value::Object(synthetic),
            tool_path.as_deref(),
            usize::MAX,
        );
    }
}

pub(crate) fn apply_tool_result_to_entries(
    entries: &mut [UiEntry],
    id: &str,
    name: &str,
    result: &str,
    details: Option<&serde_json::Value>,
    is_error: bool,
) -> bool {
    let Some(UiEntry::Tool {
        args_preview,
        tool_path,
        write_content,
        output,
        is_error: err,
        done,
        display_diff,
        ..
    }) = find_tool_mut(entries, id)
    else {
        return false;
    };

    use crate::tool_display::{extract_mcp_args_value, is_mcp_tool_name, mcp_tool_body};

    if name == "run_code"
        && let Some(human) = humanize_run_code_details(details, result)
    {
        *output = human;
        *done = true;
        *err = is_error;
        return true;
    }

    if let Some(truncated_display) = extract_truncated_tool_display(result) {
        *output = truncated_display;
    } else if is_mcp_tool_name(name) {
        let args = extract_mcp_args_value(output);
        *output = mcp_tool_body(args.as_ref(), Some(result));
    } else if let Some(human) = humanize_tool_result_for_tui(name, result, is_error) {
        match name {
            "write" | "edit" | "read" => *output = human,
            "bash" | "shell" if output.is_empty() => *output = human,
            _ if output.is_empty() => *output = human,
            _ => {}
        }
    } else if let Some(notice) = extract_full_output_notice(result) {
        if !output.contains("[Full output:") {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&notice);
        }
    } else if output.is_empty() {
        *output = result.to_string();
    }

    if !is_error
        && output_looks_like_machine_json(output)
        && let Some(human) = humanize_tool_result_for_tui(name, result, false)
    {
        *output = human;
    }
    if is_edit_tool(name)
        && !is_error
        && let Some(diff) = extract_display_diff(result)
    {
        *display_diff = Some(diff);
    }

    refresh_header_from_result_path(name, result, args_preview, tool_path, write_content);

    ensure_visible_failure_line(output, result, is_error, is_mcp_tool_name(name));

    if is_edit_tool(name)
        && let Some(diff) = display_diff.as_ref()
        && let Some(range) = extract_line_range_from_display_diff(diff)
    {
        *args_preview = merge_path_preview_with_range(args_preview, &range);
    }

    *err = is_error;
    *done = true;
    true
}

fn humanize_run_code_details(details: Option<&serde_json::Value>, result: &str) -> Option<String> {
    let details = details?.as_object()?;
    let mut lines = Vec::new();
    if let Some(summary) = details
        .get("script_summary")
        .and_then(serde_json::Value::as_str)
        && !summary.is_empty()
    {
        lines.push(format!("script: {summary}"));
    }
    if let Some(ms) = details
        .get("elapsed_ms")
        .and_then(serde_json::Value::as_u64)
    {
        lines.push(format!("elapsed: {ms}ms"));
    }
    if details
        .get("truncated")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        lines.push("[truncated]".into());
    }
    if let Some(kind) = details
        .get("error_kind")
        .and_then(serde_json::Value::as_str)
    {
        lines.push(format!("error: {kind}"));
    }
    if let Some(calls) = details
        .get("nested_calls")
        .and_then(serde_json::Value::as_array)
        && !calls.is_empty()
    {
        lines.push(format!("nested calls: {}", calls.len()));
        for call in calls {
            let name = call
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("?");
            let ok = call
                .get("ok")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let elapsed = call
                .get("elapsed_ms")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let args = call
                .get("args_preview")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let status = if ok { "ok" } else { "err" };
            let kind = call
                .get("error_kind")
                .and_then(serde_json::Value::as_str)
                .map(|kind| format!(" [{kind}]"))
                .unwrap_or_default();
            lines.push(format!("  - {name}({args}) {status}{kind} {elapsed}ms"));
        }
    }
    if !result.trim().is_empty() {
        lines.push(String::new());
        lines.push(result.to_string());
    }
    Some(lines.join("\n"))
}

pub(crate) fn project_ask_result(
    entries: &mut Vec<UiEntry>,
    id: &str,
    result: &str,
    is_error: bool,
) {
    let (phase, summary, detail_lines) = humanize_ask_result(result, is_error);
    upsert_ask_entry(entries, id, phase, &summary, detail_lines);
}

pub(crate) fn upsert_ask_entry(
    entries: &mut Vec<UiEntry>,
    id: &str,
    phase: AskPhase,
    summary: &str,
    detail_lines: Vec<String>,
) {
    if let Some(UiEntry::Ask {
        summary: s,
        detail_lines: d,
        phase: p,
        expanded,
        ..
    }) = entries.iter_mut().rev().find(|e| match e {
        UiEntry::Ask { id: tid, .. } => tid == id,
        _ => false,
    }) {
        *s = summary.to_string();
        *d = detail_lines;
        *p = phase;
        *expanded = false;
        return;
    }
    entries.push(UiEntry::Ask {
        id: id.to_string(),
        summary: summary.to_string(),
        detail_lines,
        phase,
        expanded: false,
    });
}
