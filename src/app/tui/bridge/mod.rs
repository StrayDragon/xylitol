//! XyEvent → UI-only model seam (c465 / c494).
//!
//! Render / `UiRoot` MUST consume [`UiModel`] only — never match [`XyEvent`].
//! Event-family logic lives in per-family handler modules; [`apply_xy_event`] remains the sole entry.

mod handlers;
mod model;
mod preview;
pub(crate) mod session_tree;

pub(crate) use model::trailing_aborted_note;
pub use model::{
    AskPhase, BashBlockStatus, CompactionBlockStatus, QueueBadge, STREAMING_THINK_ID, UiEntry,
    UiModel, UiPhase, allocate_thinking_id,
};
pub use preview::extract_display_diff;
/// att13：折叠态工具人话摘要（窄导出，供 BDD 直驱纯函数合约）。
pub use preview::human_tool_args_preview;
pub(crate) use preview::{
    display_tool_title, extract_full_output_notice, extract_line_range_from_display_diff,
    extract_result_path, extract_tool_path, extract_truncated_tool_display,
    human_tool_args_preview_with_path, humanize_ask_result, humanize_tool_result_for_tui,
    merge_path_preview_with_range, output_looks_like_machine_json, preview_is_downgrade,
    preview_lacks_real_path,
};
/// att12/att18：travel 重建 seam（history rebuild 的正常路径入口）。
pub use session_tree::{rebuild_scrollback_from_travel, travel_history_note};

use serde_json::Value;

use crate::app::core::driver::XyEvent;
use crate::app::tool_display::{is_ask_tool, is_edit_tool, is_write_tool};
use crate::protocol::message::AgentMessage;

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

/// Upsert a pending tool row from streaming intent (MessageUpdate) or execution start.
pub(crate) fn upsert_tool_entry(model: &mut UiModel, id: &str, name: &str, args: &Value) {
    use crate::app::tool_display::{is_mcp_tool_name, mcp_tool_body};

    let fresh_path = extract_tool_path(args);
    // c2435: model-requested bound (bash/grep/find only) becomes header note.
    let timeout_secs = matches!(name, "bash" | "grep" | "find")
        .then(|| args.get("timeout"))
        .flatten()
        .and_then(Value::as_i64)
        .filter(|n| *n > 0)
        .map(|n| n.min(crate::protocol::MAX_TOOL_TIMEOUT_SECS as i64) as u64);
    let write_content = is_write_tool(name)
        .then(|| {
            args.get("content")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .flatten()
        .filter(|s| !s.is_empty());
    let mcp = is_mcp_tool_name(name);
    // Header: no args section for mcp (body owns `args:` — avoids `{}` / compact JSON redundancy).
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
        // Refresh pretty call args while still pending (no streamed result yet).
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
        args_preview: preview,
        tool_path,
        timeout_secs,
        write_content,
        display_diff: None,
        output: mcp_pending_body.unwrap_or_default(),
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
    use crate::protocol::message::{AgentPart, LlmMessage};

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
            if is_ask_tool(name) {
                if !model
                    .entries
                    .iter()
                    .rev()
                    .any(|e| matches!(e, UiEntry::Ask { id: tid, .. } if tid == id))
                {
                    model.entries.push(UiEntry::Ask {
                        id: id.to_string(),
                        summary: "Ask · 等待回答…".into(),
                        detail_lines: vec![],
                        phase: AskPhase::Waiting,
                        expanded: false,
                    });
                }
            } else {
                upsert_tool_entry(model, id, name, arguments);
            }
        }
    }
}

pub(crate) fn find_tool_mut<'a>(entries: &'a mut [UiEntry], id: &str) -> Option<&'a mut UiEntry> {
    entries.iter_mut().rev().find(|e| match e {
        UiEntry::Tool { id: tid, .. } => tid == id,
        _ => false,
    })
}

/// c2440: terminal tool failures must be VISIBLE in the block body, not only
/// tinted on the rail — streamed output would otherwise hide the error line
/// (bash timeout keeps its partial stdout).
fn ensure_visible_failure_line(output: &mut String, result: &str, is_error: bool, is_mcp: bool) {
    if !is_error || is_mcp || output.contains(result) {
        return;
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str(result);
}

/// pi ToolExecutionComponent: updateResult refreshes result body/tint only —
/// call header stays from streaming args. Never rebuild preview from an empty
/// synthetic (that wiped bash `$ cmd` / write|edit paths after done).
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

/// Fill an existing Tool row with End semantics (live `ToolExecutionEnd` + rebuild merge).
///
/// Returns `false` when no Tool with `id` exists (caller may push an orphan stub then retry).
pub(crate) fn apply_tool_result_to_entries(
    entries: &mut [UiEntry],
    id: &str,
    name: &str,
    result: &str,
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

    use crate::app::tool_display::{extract_mcp_args_value, is_mcp_tool_name, mcp_tool_body};

    if let Some(truncated_display) = extract_truncated_tool_display(result) {
        // att16: drop streamed full buffer; keep truncated view + Full output footer.
        *output = truncated_display;
    } else if is_mcp_tool_name(name) {
        // c1460: rebuild — drop any streamed raw append; no content extract.
        let args = extract_mcp_args_value(output);
        *output = mcp_tool_body(args.as_ref(), Some(result));
    } else if let Some(human) = humanize_tool_result_for_tui(name, result, is_error) {
        // write/edit/read always replace; bash only when no live stream yet.
        match name {
            "write" | "edit" | "read" => *output = human,
            "bash" | "shell" if output.is_empty() => *output = human,
            _ if output.is_empty() => *output = human,
            _ => {}
        }
    } else if let Some(notice) = extract_full_output_notice(result) {
        // Streaming bash kept live chunks; append pi Full output footer once.
        if !output.contains("[Full output:") {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&notice);
        }
    } else if output.is_empty() {
        *output = result.to_string();
    }

    // Safety net: never leave built-in success JSON remnants in the TUI body.
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

    // pi ToolExecutionComponent: updateResult refreshes result body/tint only —
    // call header stays from streaming args. Never rebuild preview from an empty
    // synthetic (that wiped bash `$ cmd` / write|edit paths after done).
    refresh_header_from_result_path(name, result, args_preview, tool_path, write_content);

    // c2440: terminal tool failures must be VISIBLE in the block body, not
    // only tinted on the rail — streamed output would otherwise hide the
    // error line (bash timeout keeps its partial stdout).
    ensure_visible_failure_line(output, result, is_error, is_mcp_tool_name(name));

    // Edit line-range after path backfill so `:N-M` is not wiped.
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

/// Append a user scrollback row; skip if it duplicates the trailing user entry
/// (e.g. idle `begin_run` already seeded the same prompt).
///
/// Steer / follow-up inject MUST still add a bubble even when the text matches
/// the previous user row (`ati45`).
pub(crate) fn push_user_entry_dedup(model: &mut UiModel, text: String) {
    let same_trailing_user = matches!(
        model.entries.last(),
        Some(UiEntry::User { text: last }) if last == &text
    );
    let queued_inject = model.pending_steer.iter().any(|s| s == &text)
        || model.pending_follow_up.iter().any(|s| s == &text);
    if same_trailing_user && !queued_inject {
        return;
    }
    model.entries.push(UiEntry::User { text });
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
