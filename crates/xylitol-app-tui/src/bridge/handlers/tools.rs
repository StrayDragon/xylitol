use crate::bridge::session_tree::sync_todo_checklist;
use crate::bridge::{
    AskPhase, UiEntry, UiModel, UiPhase, apply_tool_result_to_entries, find_tool_mut,
    project_ask_result, upsert_ask_entry, upsert_tool_entry,
};
use crate::driver::XyEvent;
use crate::tool_display::is_ask_tool;

pub fn apply_tools_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::TodoUpdated { list } => {
            sync_todo_checklist(model, list.clone());
            true
        }
        XyEvent::WorkflowUpdated { .. } => true,
        XyEvent::ToolExecutionStart { id, name, args } => {
            apply_tool_start(model, id, name, args);
            true
        }
        XyEvent::ToolExecutionUpdate { id, output } => {
            apply_tool_update(model, id, output);
            true
        }
        XyEvent::ToolExecutionEnd {
            id,
            name,
            result,
            details,
            is_error,
        } => {
            apply_tool_end(model, id, name, result, details.as_ref(), *is_error);
            true
        }
        _ => false,
    }
}

fn apply_tool_start(model: &mut UiModel, id: &str, name: &str, args: &serde_json::Value) {
    model.flush_streaming();
    if is_ask_tool(name) {
        upsert_ask_entry(
            &mut model.entries,
            id,
            AskPhase::Waiting,
            "Ask · 等待回答…",
            vec![],
        );
    } else {
        upsert_tool_entry(model, id, name, args);
    }
    model.set_busy_status(format!("Running {name}"));
}

fn apply_tool_update(model: &mut UiModel, id: &str, output: &str) {
    if let Some(UiEntry::Tool {
        name, output: buf, ..
    }) = find_tool_mut(&mut model.entries, id)
    {
        use crate::tool_display::is_mcp_tool_name;
        if is_mcp_tool_name(name) && buf.starts_with("args:") {
            return;
        }
        buf.push_str(output);
    }
}

fn apply_tool_end(
    model: &mut UiModel,
    id: &str,
    name: &str,
    result: &str,
    details: Option<&serde_json::Value>,
    is_error: bool,
) {
    if is_ask_tool(name) {
        project_ask_result(&mut model.entries, id, result, is_error);
    } else {
        let _ =
            apply_tool_result_to_entries(&mut model.entries, id, name, result, details, is_error);
    }
    if model.phase == UiPhase::Busy {
        model.set_busy_status("Working");
    }
}
