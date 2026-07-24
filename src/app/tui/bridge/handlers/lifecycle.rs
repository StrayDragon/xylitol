//! Queue / compaction / retry / error / metadata events.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{UiEntry, UiModel, UiPhase, trailing_aborted_note};

pub fn apply_lifecycle_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::QueueUpdate {
            steer_count,
            follow_up_count,
        } => {
            model.sync_queue(*steer_count, *follow_up_count);
            true
        }
        XyEvent::CompactionStart { reason } => {
            model.entries.push(UiEntry::System {
                text: format!("compaction: {reason}"),
            });
            model.set_busy_status("Compacting");
            true
        }
        XyEvent::CompactionEnd { aborted, .. } => {
            let text = if *aborted {
                "compaction aborted"
            } else {
                "compaction complete"
            };
            model.entries.push(UiEntry::System { text: text.into() });
            // Sticky Compacting would block layout status; restore like ToolExecutionEnd.
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        XyEvent::AutoRetryStart {
            attempt,
            max_retries,
            ..
        } => {
            model.set_busy_status(format!("Retry {attempt}/{max_retries}"));
            true
        }
        XyEvent::AutoRetryEnd { success, attempt } => {
            if !*success {
                model.entries.push(UiEntry::System {
                    text: format!("retry failed (attempt {attempt})"),
                });
            }
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        XyEvent::Error(msg) => {
            // Esc abort used to emit Error("aborted"); treat as cancel note + idle
            // so a sticky Error wall cannot block further conversation (c482 / c665).
            if msg == "aborted" {
                // c1595: keep partial (flush) + footer; do not wipe already-committed assistant.
                model.flush_streaming();
                if !trailing_aborted_note(&model.entries) {
                    model.entries.push(UiEntry::System {
                        text: "Operation aborted".into(),
                    });
                }
                model.streaming_thinking.clear();
                model.streaming_assistant.clear();
                model.current_role = None;
                if model.queue.follow_up_count == 0 {
                    model.phase = UiPhase::Idle;
                    model.status = None;
                }
            } else {
                model.entries.push(UiEntry::Error { text: msg.clone() });
            }
            true
        }
        XyEvent::ModelSelect { .. }
        | XyEvent::ThinkingLevelChanged { .. }
        | XyEvent::SessionInfoChanged { .. } => {
            log::debug!(target: "xylitol::tui", "XyEvent ignored by bridge (metadata) event={}", event.description());
            true
        }
        _ => false,
    }
}
