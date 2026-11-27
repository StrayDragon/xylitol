use xylitol_plugin_sdk::message::{AgentMessage, XyLlmMessage, XyStopReason};

use crate::bridge::{
    UiEntry, UiModel, UiPhase, push_user_entry_dedup, sync_tool_intent_from_message,
};
use crate::driver::XyEvent;

fn is_turn_abort_end_mark(messages: &[AgentMessage]) -> bool {
    matches!(
        messages.last(),
        Some(AgentMessage::Llm(XyLlmMessage::AssistantMessage {
            content,
            stop_reason: Some(XyStopReason::Aborted),
            error_message: None,
            ..
        })) if content.is_empty()
    )
}

pub fn apply_agent_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::AgentStart { .. } => {
            model.set_busy_placeholder("Working");
            true
        }
        XyEvent::AgentEnd { messages, reason } => {
            model.flush_streaming();
            if let Some(reason) = reason {
                if is_turn_abort_end_mark(messages) {
                    model.entries.push(UiEntry::TurnAbortMark);
                }
                model.note_agent_end_reason(*reason);
            }
            model.maybe_idle_after_agent_end();
            true
        }
        XyEvent::StepStart { .. } => {
            if model.phase == UiPhase::Busy {
                model.set_busy_status("Working");
            }
            true
        }
        XyEvent::StepEnd { .. } => {
            log::trace!(
                target: "xylitol::tui",
                "StepEnd (intermediate); UI stays busy"
            );
            true
        }
        XyEvent::MessageStart { role, message } => {
            model.flush_streaming();
            model.current_role = Some(role.clone());
            if role == "assistant" {
                model.set_busy_status("Drafting reply");
            } else if role == "user" {
                if let Some(msg) = message {
                    let text = msg.text();
                    if !text.trim().is_empty() {
                        push_user_entry_dedup(model, text);
                    }
                }
                if model.phase == UiPhase::Busy {
                    model.set_busy_placeholder("Working");
                }
            }
            true
        }
        XyEvent::MessageUpdate { message, .. } => {
            if let Some(msg) = message {
                sync_tool_intent_from_message(model, msg);
            }
            true
        }
        XyEvent::MessageEnd { .. } => {
            model.flush_streaming();
            model.current_role = None;
            true
        }
        _ => false,
    }
}
