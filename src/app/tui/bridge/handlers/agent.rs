//! Agent / turn / message lifecycle events.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{
    UiModel, UiPhase, push_user_entry_dedup, sync_tool_intent_from_message,
};

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
            // Intermediate ReAct boundary — keep busy; do not clear streaming.
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        XyEvent::TurnEnd { .. } => {
            // MUST NOT treat as user-visible round end / idle reset.
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
                // Steer / follow-up inject (and any future user MessageStart):
                // commit into scrollback so queued strip can leave without losing text.
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
            // Text/thinking top-level fields are snapshots — TextDelta/ThinkingDelta
            // already stream increments (must not re-apply). ToolCall parts in
            // `message` are intent-only: flush streaming first (order), then
            // mount/update Tool chrome (c1260).
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
