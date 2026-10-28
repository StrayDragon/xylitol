use crate::bridge::preview::humanize_ask_result;
use crate::bridge::session_tree::sync_todo_checklist;
use crate::bridge::{
    AskPhase, ModeStripState, UiEntry, UiModel, UiPhase, apply_tool_result_to_entries,
    find_tool_mut, upsert_tool_entry,
};
use crate::driver::XyEvent;
use crate::tool_display::is_ask_tool;

pub fn apply_tools_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::TodoUpdated { list } => {
            sync_todo_checklist(model, list.clone());
            true
        }
        XyEvent::PlanModeChanged { mode, plan_id } => {
            model.mode_strip = mode_strip_for(mode, plan_id);
            true
        }
        XyEvent::PlanDocUpdated { .. } | XyEvent::WorkflowUpdated { .. } => true,
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
            is_error,
        } => {
            apply_tool_end(model, id, name, result, *is_error);
            true
        }
        _ => false,
    }
}

fn mode_strip_for(
    mode: &xylitol_plugin_sdk::session::PlanModeKind,
    plan_id: &Option<String>,
) -> Option<ModeStripState> {
    match mode {
        xylitol_plugin_sdk::session::PlanModeKind::Plan => Some(ModeStripState {
            label: "plan".into(),
        }),
        xylitol_plugin_sdk::session::PlanModeKind::Execute => {
            plan_id.as_ref().map(|id| ModeStripState {
                label: format!("execute · {id}"),
            })
        }
    }
}

fn apply_tool_start(model: &mut UiModel, id: &str, name: &str, args: &serde_json::Value) {
    model.flush_streaming();
    if is_ask_tool(name) {
        upsert_ask_entry(model, id, AskPhase::Waiting, "Ask · 等待回答…", vec![]);
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

fn apply_tool_end(model: &mut UiModel, id: &str, name: &str, result: &str, is_error: bool) {
    if is_ask_tool(name) {
        let (phase, summary, detail) = humanize_ask_result(result, is_error);
        upsert_ask_entry(model, id, phase, &summary, detail);
    } else {
        let _ = apply_tool_result_to_entries(&mut model.entries, id, name, result, is_error);
    }
    if model.phase == UiPhase::Busy {
        model.status = Some("Working".into());
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
