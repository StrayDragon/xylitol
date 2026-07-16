//! XyEvent → UI-only model seam (c465 / c494).
//!
//! Render / `UiRoot` MUST consume [`UiModel`] only — never match [`XyEvent`].
//! Event-family logic lives in per-family handler modules; [`apply_xy_event`] remains the sole entry.

mod handlers;
mod model;
pub(crate) mod session_tree;

pub(crate) use model::trailing_aborted_note;
pub use model::{BashBlockStatus, QueueBadge, UiEntry, UiModel, UiPhase};

use serde_json::Value;

use crate::app::core::driver::XyEvent;

/// Single seam: translate one [`XyEvent`] into UI-only mutations.
///
/// Unhandled / metadata variants are logged and ignored — never panic.
/// Family handlers live in per-family modules; this remains the only public entry.
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

pub(crate) fn compact_json_preview(value: &Value, max_chars: usize) -> String {
    let raw = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if raw.chars().count() <= max_chars {
        return raw;
    }
    let truncated: String = raw.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{truncated}…")
}

pub(crate) fn find_tool_mut<'a>(entries: &'a mut [UiEntry], id: &str) -> Option<&'a mut UiEntry> {
    entries.iter_mut().rev().find(|e| match e {
        UiEntry::Tool { id: tid, .. } => tid == id,
        _ => false,
    })
}

/// Append a user scrollback row; skip if it duplicates the trailing user entry
/// (e.g. idle `begin_run` already seeded the same prompt).
pub(crate) fn push_user_entry_dedup(model: &mut UiModel, text: String) {
    if let Some(UiEntry::User { text: last }) = model.entries.last()
        && last == &text
    {
        return;
    }
    model.entries.push(UiEntry::User { text });
}

/// Pull `display_diff` from edit-tool JSON result (shape is intentionally fragile).
pub fn extract_display_diff(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    value
        .get("display_diff")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

pub(crate) fn extract_edit_path(result: &str) -> Option<String> {
    let value: Value = serde_json::from_str(result).ok()?;
    value
        .get("path")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::AgentMessage;

    fn assistant_end() -> XyEvent {
        XyEvent::AgentEnd {
            messages: Vec::<AgentMessage>::new(),
        }
    }

    #[test]
    fn text_deltas_commit_on_message_end() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
        );
        apply_xy_event(&mut model, &XyEvent::TextDelta("Hello".into()));
        apply_xy_event(&mut model, &XyEvent::TextDelta("!".into()));
        apply_xy_event(
            &mut model,
            &XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        );
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Assistant { text } if text == "Hello!"))
        );
        assert_eq!(model.streaming_assistant, "");
    }

    #[test]
    fn turn_end_does_not_idle() {
        let mut model = UiModel::new();
        model.begin_run("x");
        apply_xy_event(&mut model, &XyEvent::TurnStart { turn_index: 0 });
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "1".into(),
                name: "read".into(),
                args: Value::Null,
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "1".into(),
                name: "read".into(),
                result: "ok".into(),
                is_error: false,
            },
        );
        apply_xy_event(&mut model, &XyEvent::TurnEnd { turn_index: 0 });
        assert_eq!(model.phase, UiPhase::Busy);
        assert!(model.status.is_some());
    }

    #[test]
    fn agent_end_idles_when_follow_up_empty() {
        let mut model = UiModel::new();
        model.begin_run("x");
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 0,
                follow_up_count: 0,
            },
        );
        apply_xy_event(&mut model, &assistant_end());
        assert_eq!(model.phase, UiPhase::Idle);
        assert_eq!(model.status, None);
    }

    #[test]
    fn agent_end_stays_busy_with_follow_up() {
        let mut model = UiModel::new();
        model.begin_run("x");
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 0,
                follow_up_count: 1,
            },
        );
        apply_xy_event(&mut model, &assistant_end());
        assert_eq!(model.phase, UiPhase::Busy);
        assert_eq!(model.queue.follow_up_count, 1);
    }

    #[test]
    fn queue_update_sets_badge() {
        let mut model = UiModel::new();
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 2,
                follow_up_count: 3,
            },
        );
        assert_eq!(
            model.queue,
            QueueBadge {
                steer_count: 2,
                follow_up_count: 3
            }
        );
    }

    #[test]
    fn queue_update_trims_pending_strip_fifo() {
        let mut model = UiModel::new();
        model.enqueue_steer_strip("a".into());
        model.enqueue_steer_strip("b".into());
        model.enqueue_follow_up_strip("c".into());
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 1,
                follow_up_count: 0,
            },
        );
        assert_eq!(model.pending_steer, vec!["b".to_string()]);
        assert!(model.pending_follow_up.is_empty());
    }

    #[test]
    fn user_message_start_commits_steer_to_scrollback() {
        use crate::domain::message::AgentMessage;

        let mut model = UiModel::new();
        model.begin_run("hello");
        model.enqueue_steer_strip("nudge".into());
        apply_xy_event(
            &mut model,
            &XyEvent::MessageStart {
                role: "user".into(),
                message: Some(AgentMessage::user("nudge")),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::MessageEnd {
                role: "user".into(),
                message: Some(AgentMessage::user("nudge")),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::QueueUpdate {
                steer_count: 0,
                follow_up_count: 0,
            },
        );
        assert!(model.pending_steer.is_empty());
        let users: Vec<_> = model
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::User { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(users, ["hello", "nudge"]);
    }

    #[test]
    fn aborted_error_is_system_note_and_idles() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(&mut model, &XyEvent::Error("aborted".into()));
        assert_eq!(model.phase, UiPhase::Idle);
        assert!(model.status.is_none());
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "Aborted"))
        );
        assert!(
            !model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::Error { .. }))
        );
    }

    #[test]
    fn note_bash_cancelled_does_not_emit_aborted() {
        let mut model = UiModel::new();
        model.begin_bash_block("sleep 1", false);
        model.note_bash_cancelled();
        assert!(model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Bash {
                status: BashBlockStatus::Cancelled,
                output,
                ..
            } if output.contains("(cancelled)")
        )));
        assert!(
            !model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "Aborted"))
        );
    }

    #[test]
    fn note_user_abort_allows_second_abort_after_new_command() {
        let mut model = UiModel::new();
        model.note_user_abort();
        model.entries.push(UiEntry::System {
            text: "$ sleep 2".into(),
        });
        model.note_user_abort();
        let n = model
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::System { text } if text == "Aborted"))
            .count();
        assert_eq!(
            n, 2,
            "each bang abort must show Aborted: {:?}",
            model.entries
        );
    }

    #[test]
    fn note_user_abort_dedupes_with_error_aborted() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        model.note_user_abort();
        apply_xy_event(&mut model, &XyEvent::Error("aborted".into()));
        let n = model
            .entries
            .iter()
            .filter(
                |e| matches!(e, UiEntry::System { text } if text == "Aborted" || text == "aborted"),
            )
            .count();
        assert_eq!(n, 1, "must not duplicate Aborted notes");
        assert_eq!(model.phase, UiPhase::Idle);
    }

    #[test]
    fn edit_tool_end_extracts_display_diff() {
        let mut model = UiModel::new();
        model.begin_run("edit please");
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionStart {
                id: "e1".into(),
                name: "edit".into(),
                args: Value::Null,
            },
        );
        let result = r#"{"success":true,"path":"src/a.rs","display_diff":"1 1 | fn main() {}","diff":"---"}"#;
        apply_xy_event(
            &mut model,
            &XyEvent::ToolExecutionEnd {
                id: "e1".into(),
                name: "edit".into(),
                result: result.into(),
                is_error: false,
            },
        );
        assert!(model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Diff {
                summary,
                display_diff
            } if summary.contains("src/a.rs") && display_diff.contains("fn main")
        )));
    }

    #[test]
    fn metadata_events_do_not_panic() {
        let mut model = UiModel::new();
        apply_xy_event(
            &mut model,
            &XyEvent::ModelSelect {
                provider: "openai".into(),
                model_id: "gpt".into(),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::ThinkingLevelChanged {
                level: "high".into(),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::SessionInfoChanged {
                key: "k".into(),
                value: Value::Null,
            },
        );
        assert!(model.entries.is_empty());
        assert_eq!(model.phase, UiPhase::Idle);
    }

    #[test]
    fn message_update_does_not_duplicate_deltas() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(&mut model, &XyEvent::TextDelta("Hello".into()));
        apply_xy_event(
            &mut model,
            &XyEvent::MessageUpdate {
                text: "Hello".into(),
                thinking: None,
                message: None,
            },
        );
        apply_xy_event(&mut model, &XyEvent::TextDelta("!".into()));
        apply_xy_event(
            &mut model,
            &XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        );
        let assistants: Vec<_> = model
            .entries
            .iter()
            .filter_map(|e| match e {
                UiEntry::Assistant { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(assistants, ["Hello!"]);
    }

    #[test]
    fn compaction_start_sets_status_and_note() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionStart {
                reason: "auto: 90%".into(),
            },
        );
        assert_eq!(model.phase, UiPhase::Busy);
        assert_eq!(model.status.as_deref(), Some("Compacting"));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("auto: 90%")))
        );
    }

    #[test]
    fn compaction_end_restores_working() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionStart {
                reason: "manual".into(),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionEnd {
                result: Some("ok".into()),
                aborted: false,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Working"));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "compaction complete"))
        );
    }

    #[test]
    fn compaction_end_aborted_restores_working() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionStart {
                reason: "manual".into(),
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::CompactionEnd {
                result: None,
                aborted: true,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Working"));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text == "compaction aborted"))
        );
    }

    #[test]
    fn auto_retry_start_sets_status() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryStart {
                attempt: 2,
                max_retries: 5,
                delay_ms: 100,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Retry 2/5"));
    }

    #[test]
    fn auto_retry_end_fail_notes_and_restores() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryStart {
                attempt: 1,
                max_retries: 3,
                delay_ms: 0,
            },
        );
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryEnd {
                success: false,
                attempt: 1,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Working"));
        assert!(
            model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("retry failed")))
        );
    }

    #[test]
    fn auto_retry_end_success_restores_without_fail_note() {
        let mut model = UiModel::new();
        model.begin_run("hi");
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryStart {
                attempt: 1,
                max_retries: 3,
                delay_ms: 0,
            },
        );
        let before = model.entries.len();
        apply_xy_event(
            &mut model,
            &XyEvent::AutoRetryEnd {
                success: true,
                attempt: 1,
            },
        );
        assert_eq!(model.status.as_deref(), Some("Working"));
        assert_eq!(model.entries.len(), before);
        assert!(
            !model
                .entries
                .iter()
                .any(|e| matches!(e, UiEntry::System { text } if text.contains("retry failed")))
        );
    }
}
