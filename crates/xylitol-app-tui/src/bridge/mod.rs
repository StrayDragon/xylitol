mod handlers;
mod model;
mod preview;
pub(crate) mod session_tree;
mod tool_projection;

pub(crate) use tool_projection::{
    apply_tool_result_to_entries, find_tool_mut, project_ask_result, upsert_ask_entry,
    upsert_tool_entry,
};

pub use model::trailing_aborted_note;
pub use model::{
    AskOutcome, AskPhase, BashBlockStatus, CompactionBlockStatus, QueueBadge, STREAMING_THINK_ID,
    StreamingTailKind, UiEntry, UiModel, UiPhase, allocate_thinking_id,
};
pub(crate) use model::{CUSTOM_TYPE_CONTEXT_REMINDER, system_reminder_body, user_entry};
pub use preview::TODO_EMPTY_BODY_HINT;
pub use preview::human_tool_args_preview;
pub(crate) use preview::{display_tool_title, extract_tool_path, humanize_ask_result};
pub use session_tree::{rebuild_scrollback_from_travel, travel_history_note};

use crate::driver::XyEvent;
use crate::tool_display::is_ask_tool;
use xylitol_plugin_sdk::message::AgentMessage;

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


pub(crate) fn sync_tool_intent_from_message(model: &mut UiModel, message: &AgentMessage) {
    use xylitol_plugin_sdk::message::{AgentPart, XyLlmMessage};

    let AgentMessage::Llm(XyLlmMessage::AssistantMessage { content, .. }) = message else {
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






pub(crate) fn push_user_entry_dedup(model: &mut UiModel, text: String) {
    let entry = user_entry(text.clone());
    let same_trailing = match (&entry, model.entries.last()) {
        (UiEntry::User { text: a }, Some(UiEntry::User { text: b }))
        | (UiEntry::SystemMessage { text: a }, Some(UiEntry::SystemMessage { text: b })) => a == b,
        _ => false,
    };
    let queued_inject = model.pending_steer.iter().any(|s| s == &text)
        || model.pending_follow_up.iter().any(|s| s == &text);
    if same_trailing && !queued_inject {
        return;
    }
    model.entries.push(entry);
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
