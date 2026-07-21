//! Tool execution + edit diff extraction.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{
    UiEntry, UiModel, UiPhase, extract_display_diff, extract_full_output_notice,
    extract_line_range_from_display_diff, extract_result_path, extract_truncated_tool_display,
    find_tool_mut, human_tool_args_preview_with_path, humanize_tool_result_for_tui,
    merge_path_preview_with_range, output_looks_like_machine_json, preview_lacks_real_path,
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
                if let Some(truncated_display) = extract_truncated_tool_display(result) {
                    // att16: drop streamed full buffer; keep truncated view + Full output footer.
                    *output = truncated_display;
                } else if let Some(human) = humanize_tool_result_for_tui(name, result, *is_error) {
                    // write/edit/read always replace; bash only when no live stream yet.
                    match name.as_str() {
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
                    *output = result.clone();
                }

                // Safety net: never leave built-in success JSON chrome in the TUI body.
                if !*is_error
                    && output_looks_like_machine_json(output)
                    && let Some(human) = humanize_tool_result_for_tui(name, result, false)
                {
                    *output = human;
                }
                if *name == "edit"
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
                            usize::MAX,
                        );
                    }
                }

                // Edit line-range after path backfill so `:N-M` is not wiped.
                if *name == "edit"
                    && let Some(diff) = display_diff.as_ref()
                    && let Some(range) = extract_line_range_from_display_diff(diff)
                {
                    *args_preview = merge_path_preview_with_range(args_preview, &range);
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
