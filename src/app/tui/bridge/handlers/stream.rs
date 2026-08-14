//! Streaming text / thinking deltas.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::UiModel;

pub fn apply_stream_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::TextDelta(text) => {
            model.streaming_assistant.push_str(text);
            model.set_busy_status("Drafting reply");
            true
        }
        XyEvent::ThinkingDelta(text) => {
            if model.streaming_thinking.is_empty() {
                model.thinking_started_at = Some(std::time::Instant::now());
            }
            model.streaming_thinking.push_str(text);
            model.set_busy_status("Thinking");
            true
        }
        _ => false,
    }
}
