//! XyEvent → UI-only model seam (c465).
//!
//! Render / `UiRoot` MUST consume [`UiModel`] only — never match [`XyEvent`].

use serde_json::Value;

use crate::app::core::driver::XyEvent;

/// User-visible busy vs idle (idle ⇒ status row is 0 lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPhase {
    Idle,
    Busy,
}

/// Queue badge counts from [`XyEvent::QueueUpdate`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueBadge {
    pub steer_count: usize,
    pub follow_up_count: usize,
}

/// One scrollback / transcript entry — UI-only, no domain types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEntry {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    Thinking {
        text: String,
    },
    Tool {
        id: String,
        name: String,
        args_preview: String,
        output: String,
        is_error: bool,
        done: bool,
    },
    Diff {
        summary: String,
        display_diff: String,
    },
    System {
        text: String,
    },
    Error {
        text: String,
    },
}

/// Product TUI state produced solely by [`apply_xy_event`] / [`UiModel::begin_run`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiModel {
    pub entries: Vec<UiEntry>,
    pub phase: UiPhase,
    pub queue: QueueBadge,
    /// Local copies of queued steer texts for chrome (pi `Steering:` lines).
    pub pending_steer: Vec<String>,
    /// Local copies of queued follow-up texts for chrome (pi `Follow-up:` lines).
    pub pending_follow_up: Vec<String>,
    /// Busy-only short status; [`None`] when idle (chrome: 0 rows).
    pub status: Option<String>,
    /// In-progress assistant text (not yet committed as an entry).
    streaming_assistant: String,
    /// In-progress thinking text.
    streaming_thinking: String,
    current_role: Option<String>,
}

impl Default for UiModel {
    fn default() -> Self {
        Self::new()
    }
}

impl UiModel {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            phase: UiPhase::Idle,
            queue: QueueBadge::default(),
            pending_steer: Vec::new(),
            pending_follow_up: Vec::new(),
            status: None,
            streaming_assistant: String::new(),
            streaming_thinking: String::new(),
            current_role: None,
        }
    }

    /// Align badge counts and trim chrome message lists from the front (FIFO).
    pub fn sync_queue(&mut self, steer_count: usize, follow_up_count: usize) {
        self.queue = QueueBadge {
            steer_count,
            follow_up_count,
        };
        if self.pending_steer.len() > steer_count {
            let drop_n = self.pending_steer.len() - steer_count;
            self.pending_steer.drain(..drop_n);
        }
        if self.pending_follow_up.len() > follow_up_count {
            let drop_n = self.pending_follow_up.len() - follow_up_count;
            self.pending_follow_up.drain(..drop_n);
        }
    }

    /// Enqueue a steer message for chrome + badge (host local; Driver follows).
    pub fn enqueue_steer_chrome(&mut self, text: String) {
        self.pending_steer.push(text);
        self.queue.steer_count = self.pending_steer.len();
    }

    /// Enqueue a follow-up message for chrome + badge (host local; Driver follows).
    pub fn enqueue_follow_up_chrome(&mut self, text: String) {
        self.pending_follow_up.push(text);
        self.queue.follow_up_count = self.pending_follow_up.len();
    }

    /// Drain chrome queues into one editor blob (pi Alt+Up restore).
    pub fn take_queued_for_editor(&mut self) -> Vec<String> {
        let mut all = Vec::new();
        all.append(&mut self.pending_steer);
        all.append(&mut self.pending_follow_up);
        self.queue = QueueBadge::default();
        all
    }

    /// Local submit / steer kickoff before the first stream event arrives.
    pub fn begin_run(&mut self, user_text: &str) {
        let trimmed = user_text.trim();
        if !trimmed.is_empty() {
            self.entries.push(UiEntry::User {
                text: trimmed.to_string(),
            });
        }
        self.phase = UiPhase::Busy;
        self.status = Some("Working".into());
    }

    /// Plain-text scrollback lines for the placeholder transcript widget.
    pub fn scrollback_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in &self.entries {
            match entry {
                UiEntry::User { text } => lines.push(format!("user: {text}")),
                UiEntry::Assistant { text } => lines.push(format!("assistant: {text}")),
                UiEntry::Thinking { text } => lines.push(format!("thinking: {text}")),
                UiEntry::Tool {
                    name,
                    output,
                    is_error,
                    done,
                    ..
                } => {
                    let state = if !done {
                        "…"
                    } else if *is_error {
                        "err"
                    } else {
                        "ok"
                    };
                    let preview: String = output.lines().take(2).collect::<Vec<_>>().join(" / ");
                    if preview.is_empty() {
                        lines.push(format!("tool:{name} [{state}]"));
                    } else {
                        lines.push(format!("tool:{name} [{state}] {preview}"));
                    }
                }
                UiEntry::Diff { summary, .. } => lines.push(format!("diff: {summary}")),
                UiEntry::System { text } => lines.push(format!("system: {text}")),
                UiEntry::Error { text } => lines.push(format!("error: {text}")),
            }
        }
        for (kind, text) in self.streaming_scrollback_tails() {
            lines.push(format!("{kind}: {text}…"));
        }
        lines
    }

    /// In-flight streaming tails for chrome scrollback (role label, text).
    pub(crate) fn streaming_scrollback_tails(&self) -> Vec<(&'static str, &str)> {
        let mut out = Vec::new();
        if !self.streaming_thinking.is_empty() {
            out.push(("thinking", self.streaming_thinking.as_str()));
        }
        if !self.streaming_assistant.is_empty() {
            out.push(("assistant", self.streaming_assistant.as_str()));
        }
        out
    }

    /// Stream ended without a clean AgentEnd — idle if no pending follow-up.
    pub fn on_stream_closed_without_agent_end(&mut self) {
        if self.phase == UiPhase::Busy && self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        }
    }

    fn flush_streaming(&mut self) {
        if !self.streaming_thinking.is_empty() {
            self.entries.push(UiEntry::Thinking {
                text: std::mem::take(&mut self.streaming_thinking),
            });
        }
        if !self.streaming_assistant.is_empty() {
            self.entries.push(UiEntry::Assistant {
                text: std::mem::take(&mut self.streaming_assistant),
            });
        }
    }

    fn set_busy_status(&mut self, status: impl Into<String>) {
        self.phase = UiPhase::Busy;
        self.status = Some(status.into());
    }

    fn maybe_idle_after_agent_end(&mut self) {
        // Follow-ups are drained inside the same Driver::run before AgentEnd on
        // the happy path. After abort, follow_up may remain — stay busy so chrome
        // can restore (c480); counts still come from QueueUpdate.
        if self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        } else {
            self.set_busy_status("Follow-up pending");
        }
    }
}

/// Single seam: translate one [`XyEvent`] into UI-only mutations.
///
/// Unhandled / metadata variants are logged and ignored — never panic.
pub fn apply_xy_event(model: &mut UiModel, event: &XyEvent) {
    match event {
        XyEvent::AgentStart { .. } => {
            model.set_busy_status("Working");
        }
        XyEvent::AgentEnd { .. } => {
            model.flush_streaming();
            model.maybe_idle_after_agent_end();
        }
        XyEvent::TurnStart { .. } => {
            // Intermediate ReAct boundary — keep busy; do not clear streaming.
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
        }
        XyEvent::TurnEnd { .. } => {
            // MUST NOT treat as user-visible round end / idle reset.
            tracing::trace!(
                target: "xylitol::tui",
                "TurnEnd (intermediate); UI stays busy"
            );
        }
        XyEvent::MessageStart { role, message } => {
            model.flush_streaming();
            model.current_role = Some(role.clone());
            if role == "assistant" {
                model.set_busy_status("Drafting reply");
            } else if role == "user" {
                // Steer / follow-up inject (and any future user MessageStart):
                // commit into scrollback so queued chrome can leave without losing text.
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
        }
        XyEvent::MessageUpdate { .. } => {
            // Accumulated snapshot — TextDelta/ThinkingDelta already stream the
            // increments; applying this would duplicate prefixes (see print mode).
        }
        XyEvent::MessageEnd { .. } => {
            model.flush_streaming();
            model.current_role = None;
        }
        XyEvent::TextDelta(text) => {
            model.streaming_assistant.push_str(text);
            model.set_busy_status("Drafting reply");
        }
        XyEvent::ThinkingDelta(text) => {
            model.streaming_thinking.push_str(text);
            model.set_busy_status("Thinking");
        }
        XyEvent::ToolExecutionStart { id, name, args } => {
            model.flush_streaming();
            let args_preview = compact_json_preview(args, 80);
            model.entries.push(UiEntry::Tool {
                id: id.clone(),
                name: name.clone(),
                args_preview,
                output: String::new(),
                is_error: false,
                done: false,
            });
            model.set_busy_status(format!("Running {name}"));
        }
        XyEvent::ToolExecutionUpdate { id, output } => {
            if let Some(UiEntry::Tool { output: buf, .. }) = find_tool_mut(&mut model.entries, id) {
                buf.push_str(output);
            }
        }
        XyEvent::ToolExecutionEnd {
            id,
            name,
            result,
            is_error,
        } => {
            if let Some(UiEntry::Tool {
                output,
                is_error: err,
                done,
                ..
            }) = find_tool_mut(&mut model.entries, id)
            {
                if output.is_empty() {
                    *output = result.clone();
                }
                *err = *is_error;
                *done = true;
            }
            if name == "edit"
                && let Some(display_diff) = extract_display_diff(result)
            {
                let summary = extract_edit_path(result)
                    .map(|p| format!("edited {p}"))
                    .unwrap_or_else(|| "edit".into());
                model.entries.push(UiEntry::Diff {
                    summary,
                    display_diff,
                });
            }
            if model.phase == UiPhase::Busy {
                model.status = Some("Working".into());
            }
        }
        XyEvent::QueueUpdate {
            steer_count,
            follow_up_count,
        } => {
            model.sync_queue(*steer_count, *follow_up_count);
        }
        XyEvent::CompactionStart { reason } => {
            model.entries.push(UiEntry::System {
                text: format!("compaction: {reason}"),
            });
            model.set_busy_status("Compacting");
        }
        XyEvent::CompactionEnd { aborted, .. } => {
            let text = if *aborted {
                "compaction aborted"
            } else {
                "compaction complete"
            };
            model.entries.push(UiEntry::System { text: text.into() });
        }
        XyEvent::AutoRetryStart {
            attempt,
            max_retries,
            ..
        } => {
            model.set_busy_status(format!("Retry {attempt}/{max_retries}"));
        }
        XyEvent::AutoRetryEnd { success, attempt } => {
            if !*success {
                model.entries.push(UiEntry::System {
                    text: format!("retry failed (attempt {attempt})"),
                });
            }
        }
        XyEvent::Error(msg) => {
            model.entries.push(UiEntry::Error { text: msg.clone() });
        }
        XyEvent::ModelSelect { .. }
        | XyEvent::ThinkingLevelChanged { .. }
        | XyEvent::SessionInfoChanged { .. } => {
            tracing::debug!(
                target: "xylitol::tui",
                event = event.description(),
                "XyEvent ignored by bridge (metadata)"
            );
        }
    }
}

fn compact_json_preview(value: &Value, max_chars: usize) -> String {
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

fn find_tool_mut<'a>(entries: &'a mut [UiEntry], id: &str) -> Option<&'a mut UiEntry> {
    entries.iter_mut().rev().find(|e| match e {
        UiEntry::Tool { id: tid, .. } => tid == id,
        _ => false,
    })
}

/// Append a user scrollback row; skip if it duplicates the trailing user entry
/// (e.g. idle `begin_run` already seeded the same prompt).
fn push_user_entry_dedup(model: &mut UiModel, text: String) {
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

fn extract_edit_path(result: &str) -> Option<String> {
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
    fn queue_update_trims_pending_chrome_fifo() {
        let mut model = UiModel::new();
        model.enqueue_steer_chrome("a".into());
        model.enqueue_steer_chrome("b".into());
        model.enqueue_follow_up_chrome("c".into());
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
        model.enqueue_steer_chrome("nudge".into());
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
}
