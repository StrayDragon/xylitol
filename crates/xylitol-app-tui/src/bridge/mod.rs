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

// ~keep bridge 家族路由导航表（短路顺序即优先级：先命中者胜，同一事件不得被两个家族同时声明）。
// ~keep 事件族 → 家族函数 → 文件 → 渲染落点：
// ~keep   agent 族 AgentStart / AgentEnd / StepStart / StepEnd / Message{Start,Update,End}
// ~keep     → apply_agent_family → handlers/agent.rs → busy 占位与状态、user 条目去重尾插、流式缓冲 flush。
// ~keep   stream 族 TextDelta / ThinkingDelta
// ~keep     → apply_stream_family → handlers/stream.rs → 流式助手缓冲与思考缓冲（STREAMING_THINK_ID）。
// ~keep   tools 族 TodoUpdated / WorkflowUpdated / ToolExecution{Start,Update,End}
// ~keep     → apply_tools_family → handlers/tools.rs → 待办栏与工具 / Ask 条目，经 tool_projection 写入
// ~keep       entries，由 layout 渲染。
// ~keep   lifecycle 族 QueueUpdate / Compaction{Start,End} / AutoRetry{Start,End} / Error /
// ~keep     SessionInfoChanged / ModelSelect / ThinkingLevelChanged / ContextTokenSettlement
// ~keep     → apply_lifecycle_family → handlers/lifecycle.rs → 队列条、通知栈、状态条。
// ~keep 完整链路：xylitol-wire event_to_wire → serve host event_to_wire → SessionEventPayload →
// ~keep   WS 下行 → app-tui driver/remote.rs → lib.rs 扇入 apply_xy_event → 上表家族 → layout。
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
    let queued_inject =
        model.queue_contains(&text) || model.awaiting_user_bubble.iter().any(|s| s == &text);
    if same_trailing && !queued_inject {
        return;
    }
    if let Some(pos) = model
        .awaiting_user_bubble
        .iter()
        .position(|row| row == &text)
    {
        model.awaiting_user_bubble.remove(pos);
    } else if model.steer_contains(&text) {
        model.pop_steer_strip_matching(&text);
    } else if model.follow_up_contains(&text) {
        model.pop_follow_up_strip_matching(&text);
    }
    model.forget_queue_item_id(&text);
    model.entries.push(entry);
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
