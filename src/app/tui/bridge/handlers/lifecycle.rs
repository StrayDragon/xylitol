//! Queue / compaction / retry / error / metadata events.

use crate::app::core::driver::XyEvent;
use crate::app::tui::bridge::{
    CompactionBlockStatus, UiEntry, UiModel, UiPhase, trailing_aborted_note,
};

pub fn apply_lifecycle_family(model: &mut UiModel, event: &XyEvent) -> bool {
    match event {
        XyEvent::QueueUpdate {
            steer_count,
            follow_up_count,
        } => {
            model.sync_queue(*steer_count, *follow_up_count);
            true
        }
        XyEvent::CompactionStart { .. } => {
            push_compaction_pending(model);
            model.set_busy_status("Compacting");
            true
        }
        XyEvent::CompactionEnd {
            aborted,
            error_message,
            summary,
            tokens_before,
            ..
        } => {
            if *aborted {
                finish_compaction(
                    model,
                    CompactionBlockStatus::Aborted,
                    String::new(),
                    0,
                    Some("compaction aborted".into()),
                );
            } else if let Some(err) = error_message {
                finish_compaction(
                    model,
                    CompactionBlockStatus::Failed,
                    String::new(),
                    0,
                    Some(err.clone()),
                );
            } else {
                finish_compaction(
                    model,
                    CompactionBlockStatus::Complete,
                    summary.clone().unwrap_or_default(),
                    tokens_before.unwrap_or(0),
                    None,
                );
            }
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
                model.entries.push(UiEntry::ScrollNotice {
                    text: format!("retry failed (attempt {attempt})"),
                });
            }
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
            true
        }
        XyEvent::Error(err) => {
            // Esc abort: cancel note + idle — not a sticky Error wall (c482 / c665).
            if err.is_aborted() {
                // c1595: keep partial (flush) + footer; do not wipe already-committed assistant.
                model.flush_streaming();
                if !trailing_aborted_note(&model.entries) {
                    model.entries.push(UiEntry::ScrollNotice {
                        text: "Operation aborted".into(),
                    });
                }
                model.streaming_thinking.clear();
                model.streaming_think_id = None;
                model.streaming_assistant.clear();
                model.current_role = None;
                if model.queue.follow_up_count == 0 {
                    model.phase = UiPhase::Idle;
                    model.status = None;
                }
            } else {
                log::warn!(
                    target: "xylitol::tui",
                    "XyEvent::Error error.kind={} message={}",
                    err.kind,
                    err.message
                );
                // Non-abort kinds stay sticky Error rows; message is Display-shaped
                // when sourced from XyError (Config/Session/Provider/Tool/…).
                model.entries.push(UiEntry::Error {
                    text: err.message.clone(),
                });
            }
            true
        }
        XyEvent::ModelSelect { .. }
        | XyEvent::ThinkingLevelChanged { .. }
        | XyEvent::SessionInfoChanged { .. }
        | XyEvent::ContextTokenSettlement { .. } => {
            // Settlement is applied by host footer (c1860), not UiModel.
            log::debug!(target: "xylitol::tui", "XyEvent ignored by bridge (metadata) event={}", event.description());
            true
        }
        _ => false,
    }
}

fn push_compaction_pending(model: &mut UiModel) {
    if let Some(UiEntry::Compaction {
        status: CompactionBlockStatus::Pending,
        ..
    }) = model.entries.last()
    {
        return;
    }
    model.entries.push(UiEntry::Compaction {
        status: CompactionBlockStatus::Pending,
        summary: String::new(),
        tokens_before: 0,
        detail: None,
    });
}

fn finish_compaction(
    model: &mut UiModel,
    status: CompactionBlockStatus,
    summary: String,
    tokens_before: u64,
    detail: Option<String>,
) {
    if let Some(UiEntry::Compaction {
        status: slot_status,
        summary: slot_summary,
        tokens_before: slot_tokens,
        detail: slot_detail,
    }) = model.entries.iter_mut().rev().find(|e| {
        matches!(
            e,
            UiEntry::Compaction {
                status: CompactionBlockStatus::Pending,
                ..
            }
        )
    }) {
        *slot_status = status;
        *slot_summary = summary;
        *slot_tokens = tokens_before;
        *slot_detail = detail;
        return;
    }
    model.entries.push(UiEntry::Compaction {
        status,
        summary,
        tokens_before,
        detail,
    });
}
