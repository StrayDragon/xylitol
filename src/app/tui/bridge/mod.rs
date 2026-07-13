//! XyEvent → UI-only model seam (c465 / c494).
//!
//! Render / `UiRoot` MUST consume [`UiModel`] only — never match [`XyEvent`].
//! Event-family logic lives in per-family handler modules; [`apply_xy_event`] remains the sole entry.

mod handlers;
pub(crate) mod session_tree;

use serde_json::Value;

use crate::app::core::driver::XyEvent;

/// Decode complete UTF-8 prefix from `buf`, leaving a trailing incomplete sequence.
fn drain_utf8_prefix(buf: &mut Vec<u8>) -> String {
    match std::str::from_utf8(buf) {
        Ok(s) => {
            let out = s.to_string();
            buf.clear();
            out
        }
        Err(e) => {
            let valid = e.valid_up_to();
            if valid == 0 {
                if e.error_len().is_some() {
                    // Invalid byte — skip one and continue.
                    buf.remove(0);
                    return drain_utf8_prefix(buf);
                }
                // Incomplete sequence at end — wait for more bytes.
                return String::new();
            }
            let out = String::from_utf8_lossy(&buf[..valid]).into_owned();
            let rest = buf[valid..].to_vec();
            *buf = rest;
            out
        }
    }
}

/// True when the last scrollback line is already an abort note (same-event dedupe).
pub(crate) fn trailing_aborted_note(entries: &[UiEntry]) -> bool {
    matches!(
        entries.last(),
        Some(UiEntry::System { text }) if text == "Aborted" || text == "aborted"
    )
}

/// True when the last line is already a bang `(cancelled)` note (pi bash status).
pub(crate) fn trailing_bash_cancelled_note(entries: &[UiEntry]) -> bool {
    match entries.last() {
        Some(UiEntry::Bash {
            status: BashBlockStatus::Cancelled,
            ..
        }) => true,
        Some(UiEntry::System { text } | UiEntry::Error { text })
            if text.contains("(cancelled)") =>
        {
            true
        }
        _ => false,
    }
}

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

/// Bang / interactive bash block tint state (c668; aligns demo tool tint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BashBlockStatus {
    Pending,
    Success,
    Error,
    Cancelled,
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
    /// Interactive `!` / `!!` bash block (c668).
    Bash {
        command: String,
        status: BashBlockStatus,
        output: String,
        exclude_from_context: bool,
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
    /// Local copies of queued steer texts for queue strip (pi `Steering:` lines).
    pub pending_steer: Vec<String>,
    /// Local copies of queued follow-up texts for queue strip (pi `Follow-up:` lines).
    pub pending_follow_up: Vec<String>,
    /// Busy-only short status; [`None`] when idle (layout status: 0 rows).
    pub status: Option<String>,
    /// In-progress assistant text (not yet committed as an entry).
    pub(crate) streaming_assistant: String,
    /// In-progress thinking text.
    pub(crate) streaming_thinking: String,
    pub(crate) current_role: Option<String>,
    /// Incomplete UTF-8 bytes across bang stream chunks (c669).
    bash_utf8_pending: Vec<u8>,
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
            bash_utf8_pending: Vec::new(),
        }
    }

    /// Align badge counts and trim strip message lists from the front (FIFO).
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

    /// Enqueue a steer message for strip + badge (host local; Driver follows).
    pub fn enqueue_steer_strip(&mut self, text: String) {
        self.pending_steer.push(text);
        self.queue.steer_count = self.pending_steer.len();
    }

    /// Enqueue a follow-up message for strip + badge (host local; Driver follows).
    pub fn enqueue_follow_up_strip(&mut self, text: String) {
        self.pending_follow_up.push(text);
        self.queue.follow_up_count = self.pending_follow_up.len();
    }

    /// Drain strip queues into one editor blob (pi Alt+Up restore).
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
                UiEntry::Bash {
                    command,
                    status,
                    output,
                    ..
                } => {
                    lines.push(format!("bash[{status:?}]: $ {command}"));
                    if !output.is_empty() {
                        lines.push(output.clone());
                    }
                }
                UiEntry::System { text } => lines.push(format!("system: {text}")),
                UiEntry::Error { text } => lines.push(format!("error: {text}")),
            }
        }
        for (kind, text) in self.streaming_scrollback_tails() {
            lines.push(format!("{kind}: {text}…"));
        }
        lines
    }

    /// In-flight streaming tails for layout scrollback (role label, text).
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

    /// Immediate user Esc abort: System `Aborted` + idle status.
    /// Dedupes only a trailing abort note (Esc repeat / same-event `Error("aborted")`),
    /// not any historical `Aborted` in scrollback — each agent abort must show again.
    /// Bang Esc uses [`Self::note_bash_cancelled`] instead (pi `(cancelled)`).
    pub fn note_user_abort(&mut self) {
        if !trailing_aborted_note(&self.entries) {
            self.entries.push(UiEntry::System {
                text: "Aborted".into(),
            });
        }
        self.streaming_thinking.clear();
        self.streaming_assistant.clear();
        self.current_role = None;
        if self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        }
    }

    /// Bang Esc abort: mark last pending Bash cancelled + `(cancelled)` (pi).
    pub fn note_bash_cancelled(&mut self) {
        self.bash_utf8_pending.clear();
        let mut updated = false;
        for entry in self.entries.iter_mut().rev() {
            if let UiEntry::Bash { status, output, .. } = entry {
                if matches!(
                    *status,
                    BashBlockStatus::Pending | BashBlockStatus::Cancelled
                ) {
                    *status = BashBlockStatus::Cancelled;
                    if !output.contains("(cancelled)") {
                        if !output.is_empty() {
                            output.push('\n');
                        }
                        output.push_str("(cancelled)");
                    }
                    updated = true;
                }
                break;
            }
        }
        if !updated && !trailing_bash_cancelled_note(&self.entries) {
            self.entries.push(UiEntry::System {
                text: "(cancelled)".into(),
            });
        }
        if self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        }
    }

    /// Start a bang block (pending tint) at submit.
    pub fn begin_bash_block(&mut self, command: &str, exclude_from_context: bool) {
        self.entries.push(UiEntry::Bash {
            command: command.to_string(),
            status: BashBlockStatus::Pending,
            output: String::new(),
            exclude_from_context,
        });
        self.bash_utf8_pending.clear();
        self.phase = UiPhase::Busy;
        self.status = Some("Running".into());
    }

    /// Append live bang output bytes to the last Pending Bash block (c669).
    /// Incomplete UTF-8 sequences are held across calls.
    pub fn append_bash_output(&mut self, chunk: &[u8]) {
        self.bash_utf8_pending.extend_from_slice(chunk);
        let decoded = drain_utf8_prefix(&mut self.bash_utf8_pending);
        if decoded.is_empty() {
            return;
        }
        for entry in self.entries.iter_mut().rev() {
            if let UiEntry::Bash { status, output, .. } = entry {
                if *status == BashBlockStatus::Pending {
                    output.push_str(&decoded);
                }
                break;
            }
        }
    }

    /// Finish the last pending bang block with preformatted output body + status.
    pub fn finish_bash_block(&mut self, status: BashBlockStatus, body: String) {
        self.bash_utf8_pending.clear();
        for entry in self.entries.iter_mut().rev() {
            if let UiEntry::Bash {
                status: st, output, ..
            } = entry
            {
                if *st == BashBlockStatus::Pending || status == BashBlockStatus::Cancelled {
                    *st = status;
                    *output = body;
                }
                break;
            }
        }
    }

    /// Stream ended without a clean AgentEnd — idle if no pending follow-up.
    pub fn on_stream_closed_without_agent_end(&mut self) {
        if self.phase == UiPhase::Busy && self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        }
    }

    pub(crate) fn flush_streaming(&mut self) {
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

    pub(crate) fn set_busy_status(&mut self, status: impl Into<String>) {
        self.phase = UiPhase::Busy;
        self.status = Some(status.into());
    }

    pub(crate) fn maybe_idle_after_agent_end(&mut self) {
        // Follow-ups are drained inside the same Driver::run before AgentEnd on
        // the happy path. After abort, follow_up may remain — stay busy so layout
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
/// Family handlers live in per-family modules; this remains the only public entry.
pub fn apply_xy_event(model: &mut UiModel, event: &XyEvent) {
    if handlers::apply_agent_family(model, event)
        || handlers::apply_stream_family(model, event)
        || handlers::apply_tools_family(model, event)
        || handlers::apply_lifecycle_family(model, event)
    {
        return;
    }
    tracing::debug!(
        target: "xylitol::tui",
        event = event.description(),
        "XyEvent unhandled by bridge"
    );
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
