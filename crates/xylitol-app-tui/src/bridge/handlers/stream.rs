use crate::bridge::UiModel;
use crate::driver::XyEvent;

pub fn apply_stream_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::TextDelta(text) => {
            model.append_assistant_delta(text);
            model.set_busy_status("Drafting reply");
            true
        }
        XyEvent::ThinkingDelta(text) => {
            model.append_thinking_delta(text);
            model.set_busy_status("Thinking");
            true
        }
        _ => false,
    }
}
