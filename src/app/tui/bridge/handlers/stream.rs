//! Streaming text / thinking deltas.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::UiModel;

use super::super::STREAMING_THINK_ID;

pub fn apply_stream_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::TextDelta(text) => {
            if model.streaming_think_id.is_some() {
                model.thought_clock.stamp_end();
                model.flush_thinking_elapsed(None);
            }
            model.streaming_assistant.push_str(text);
            model.set_busy_status("Drafting reply");
            true
        }
        XyEvent::ThinkingDelta(text) => {
            if model.streaming_think_id.is_none() {
                model.streaming_think_id = Some(STREAMING_THINK_ID.into());
                model.thought_clock.stamp_start();
            }
            model.streaming_thinking.push_str(text);
            model.set_busy_status("Thinking");
            true
        }
        _ => false,
    }
}
