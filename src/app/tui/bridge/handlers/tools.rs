//! Tool execution + edit diff extraction.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{
    UiEntry, UiModel, UiPhase, extract_display_diff, extract_edit_path, find_tool_mut,
    upsert_tool_entry,
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
            if let Some(UiEntry::Tool { output: buf, .. }) = find_tool_mut(&mut model.entries, id) {
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
            if let Some(UiEntry::Tool {
                output,
                is_error: err,
                done,
                ..
            }) = find_tool_mut(&mut model.entries, id)
            {
                if output.is_empty() {
                    *output = result.clone();
                }
                *err = *is_error;
                *done = true;
            }
            if name == "edit"
                && let Some(display_diff) = extract_display_diff(result)
            {
                let summary = extract_edit_path(result)
                    .map(|p| format!("edited {p}"))
                    .unwrap_or_else(|| "edit".into());
                model.entries.push(UiEntry::Diff {
                    summary,
                    display_diff,
                });
            }
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        _ => false,
    }
}
