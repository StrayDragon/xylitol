//! Tool execution + edit diff extraction.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{
    UiEntry, UiModel, UiPhase, apply_tool_result_to_entries, find_tool_mut, upsert_tool_entry,
};

pub fn apply_tools_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::ToolExecutionStart { id, name, args } => {
            model.flush_streaming();
            upsert_tool_entry(model, id, name, args);
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
            let _ = apply_tool_result_to_entries(&mut model.entries, id, name, result, *is_error);
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        _ => false,
    }
}
