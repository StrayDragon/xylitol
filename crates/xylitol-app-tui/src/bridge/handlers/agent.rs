use crate::bridge::{UiModel, UiPhase, push_user_entry_dedup, sync_tool_intent_from_message};
use crate::driver::XyEvent;

pub fn apply_agent_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::AgentStart { .. } => {
            model.set_busy_status("Working");
            true
        }
        XyEvent::AgentEnd { .. } => {
            model.flush_streaming();
            model.maybe_idle_after_agent_end();
            true
        }
        XyEvent::TurnStart { .. } => {
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        XyEvent::TurnEnd { .. } => {
            log::trace!(
                target: "xylitol::tui",
                "TurnEnd (intermediate); UI stays busy"
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
                    model.set_busy_status("Working");
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
