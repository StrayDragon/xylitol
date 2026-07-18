//! Tool execution + edit diff extraction.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{
    UiEntry, UiModel, UiPhase, extract_display_diff, find_tool_mut, quiet_tool_success_output,
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
                display_diff,
                ..
            }) = find_tool_mut(&mut model.entries, id)
            {
                if let Some(quiet) = quiet_tool_success_output(name, result, *is_error) {
                    // Clear machine JSON result chrome; write body lives in write_content.
                    *output = quiet;
                } else if output.is_empty() {
                    *output = result.clone();
                }
                if name == "edit"
                    && !*is_error
                    && let Some(diff) = extract_display_diff(result)
                {
                    *display_diff = Some(diff);
                }
                *err = *is_error;
                *done = true;
            }
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        _ => false,
    }
}
