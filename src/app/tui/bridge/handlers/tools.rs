//! Tool execution + edit diff extraction.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{
    UiEntry, UiModel, UiPhase, extract_display_diff, extract_result_path, find_tool_mut,
    human_tool_args_preview_with_path, preview_lacks_real_path, quiet_tool_success_output,
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
                args_preview,
                tool_path,
                write_content,
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

                // pi ToolExecutionComponent: updateResult refreshes result body/tint only —
                // call header stays from streaming args. Never rebuild preview from an empty
                // synthetic (that wiped bash `$ cmd` / write|edit paths after done).
                if let Some(path) = extract_result_path(result) {
                    let weak_preview = preview_lacks_real_path(name, args_preview);
                    if tool_path.as_deref().filter(|p| !p.is_empty()).is_none() {
                        *tool_path = Some(path);
                    }
                    if weak_preview {
                        let mut synthetic = serde_json::Map::new();
                        if let Some(p) = tool_path.as_deref() {
                            synthetic
                                .insert("path".into(), serde_json::Value::String(p.to_string()));
                        }
                        if let Some(c) = write_content.as_deref() {
                            synthetic
                                .insert("content".into(), serde_json::Value::String(c.to_string()));
                        }
                        *args_preview = human_tool_args_preview_with_path(
                            name,
                            &serde_json::Value::Object(synthetic),
                            tool_path.as_deref(),
                            80,
                        );
                    }
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
