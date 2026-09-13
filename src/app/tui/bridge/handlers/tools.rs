//! Tool execution + edit diff extraction.

use crate::app::core::driver::XyEvent;
use crate::app::tool_display::is_ask_tool;
use crate::app::tui::bridge::preview::humanize_ask_result;
use crate::app::tui::bridge::session_tree::sync_todo_checklist;
use crate::app::tui::bridge::{
    AskPhase, UiEntry, UiModel, UiPhase, apply_tool_result_to_entries, find_tool_mut,
    upsert_tool_entry,
};

pub fn apply_tools_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::TodoUpdated { list } => {
            // atd13: checklist projection rides the typed event — clients never
            // re-parse tool-result strings to stay current.
            sync_todo_checklist(model, list.clone());
            true
        }
        XyEvent::ToolExecutionStart { id, name, args } => {
            model.flush_streaming();
            if is_ask_tool(name) {
                upsert_ask_entry(model, id, AskPhase::Waiting, "Ask · 等待回答…", vec![]);
            } else {
                upsert_tool_entry(model, id, name, args);
            }
            model.set_busy_status(format!("Running {name}"));
            true
        }
        XyEvent::ToolExecutionUpdate { id, output } => {
            if let Some(UiEntry::Tool {
                name, output: buf, ..
            }) = find_tool_mut(&mut model.entries, id)
            {
                use crate::app::tool_display::is_mcp_tool_name;
                // MCP body is owned by Start/End (args + pretty result). Streaming the raw
                // CallToolResult into the buffer glues onto `args:` and duplicates under result.
                if is_mcp_tool_name(name) && buf.starts_with("args:") {
                    return true;
                }
                buf.push_str(output);
            }
            true
        }
        XyEvent::ToolExecutionEnd {
            id,
            name,
            result,
            is_error,
        } => {
            if is_ask_tool(name) {
                let (phase, summary, detail) = humanize_ask_result(result, *is_error);
                upsert_ask_entry(model, id, phase, &summary, detail);
            } else {
                let _ =
                    apply_tool_result_to_entries(&mut model.entries, id, name, result, *is_error);
            }
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        _ => false,
    }
}

fn upsert_ask_entry(
    model: &mut UiModel,
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
    }) = model.entries.iter_mut().rev().find(|e| match e {
        UiEntry::Ask { id: tid, .. } => tid == id,
        _ => false,
    }) {
        *s = summary.to_string();
        *d = detail_lines;
        *p = phase;
        *expanded = false;
        return;
    }
    model.entries.push(UiEntry::Ask {
        id: id.to_string(),
        summary: summary.to_string(),
        detail_lines,
        phase,
        expanded: false,
    });
}
