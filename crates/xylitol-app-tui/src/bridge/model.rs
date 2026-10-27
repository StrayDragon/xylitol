use strum::IntoStaticStr;

use crate::utils::ThoughtClock;
use xylitol_plugin_sdk::session::TodoList;

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
                    buf.remove(0);
                    return drain_utf8_prefix(buf);
                }
                return String::new();
            }
            let out = String::from_utf8_lossy(&buf[..valid]).into_owned();
            let rest = buf[valid..].to_vec();
            *buf = rest;
            out
        }
    }
}

pub fn trailing_aborted_note(entries: &[UiEntry]) -> bool {
    matches!(
        entries.last(),
        Some(UiEntry::ScrollNotice { text })
            if text == "Aborted"
                || text == "aborted"
                || text == "Operation aborted"
                || text.starts_with("Operation aborted")
    )
}

pub(crate) fn trailing_bash_cancelled_note(entries: &[UiEntry]) -> bool {
    match entries.last() {
        Some(UiEntry::Bash {
            status: BashBlockStatus::Cancelled,
            ..
        }) => true,
        Some(UiEntry::ScrollNotice { text } | UiEntry::Error { text })
            if text.contains("(cancelled)") =>
        {
            true
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPhase {
    Idle,
    Busy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueBadge {
    pub steer_count: usize,
    pub follow_up_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BashBlockStatus {
    Pending,
    Success,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AskPhase {
    Waiting,
    Answered,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompactionBlockStatus {
    Pending,
    Complete,
    Aborted,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEntry {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    Thinking {
        id: String,
        text: String,
        elapsed_secs: Option<u64>,
    },
    Tool {
        id: String,
        name: String,
        args_preview: String,
        tool_path: Option<String>,
        write_content: Option<String>,
        display_diff: Option<String>,
        output: String,
        is_error: bool,
        done: bool,
        timeout_secs: Option<u64>,
    },
    Ask {
        id: String,
        summary: String,
        detail_lines: Vec<String>,
        phase: AskPhase,
        expanded: bool,
    },
    Diff {
        summary: String,
        display_diff: String,
    },
    Bash {
        id: String,
        command: String,
        status: BashBlockStatus,
        output: String,
        exclude_from_context: bool,
    },
    Compaction {
        status: CompactionBlockStatus,
        summary: String,
        tokens_before: u64,
        tokens_after: Option<u64>,
        detail: Option<String>,
    },
    ScrollNotice {
        text: String,
    },
    Error {
        text: String,
    },
}

pub const STREAMING_THINK_ID: &str = "live:streaming";

pub fn allocate_thinking_id<'a>(
    entries: impl IntoIterator<Item = &'a UiEntry>,
    text: &str,
) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    let short = format!("{:08x}", h.finish() as u32);
    let ordinal = entries
        .into_iter()
        .filter(|e| match e {
            UiEntry::Thinking { id, .. } => id.starts_with(&format!("{short}-")),
            _ => false,
        })
        .count();
    format!("{short}-{ordinal}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum StreamingTailKind {
    Thinking,
    Assistant,
}

impl StreamingTailKind {
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiModel {
    pub entries: Vec<UiEntry>,
    pub phase: UiPhase,
    pub queue: QueueBadge,
    pub pending_steer: Vec<String>,
    pub pending_follow_up: Vec<String>,
    pub status: Option<String>,
    pub streaming_assistant: String,
    pub(crate) streaming_thinking: String,
    pub(crate) streaming_think_id: Option<String>,
    pub(crate) thought_clock: ThoughtClock,
    pub(crate) current_role: Option<String>,
    bash_utf8_pending: Vec<u8>,
    pub todo: TodoList,
    pub has_earlier: bool,
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
            streaming_think_id: None,
            thought_clock: ThoughtClock::new(),
            current_role: None,
            bash_utf8_pending: Vec::new(),
            todo: TodoList::default(),
            has_earlier: false,
        }
    }

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

    pub fn enqueue_steer_strip(&mut self, text: String) {
        self.pending_steer.push(text);
        self.queue.steer_count = self.pending_steer.len();
    }

    pub fn enqueue_follow_up_strip(&mut self, text: String) {
        self.pending_follow_up.push(text);
        self.queue.follow_up_count = self.pending_follow_up.len();
    }

    pub fn pop_follow_up_strip_matching(&mut self, text: &str) {
        if let Some(pos) = self.pending_follow_up.iter().rposition(|t| t == text) {
            self.pending_follow_up.remove(pos);
            self.queue.follow_up_count = self.pending_follow_up.len();
        }
    }

    pub fn pop_steer_strip_matching(&mut self, text: &str) {
        if let Some(pos) = self.pending_steer.iter().rposition(|t| t == text) {
            self.pending_steer.remove(pos);
            self.queue.steer_count = self.pending_steer.len();
        }
    }

    pub fn take_queued_for_editor(&mut self) -> Vec<String> {
        let mut all = Vec::new();
        all.append(&mut self.pending_steer);
        all.append(&mut self.pending_follow_up);
        self.queue = QueueBadge::default();
        all
    }

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

    pub fn scrollback_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in &self.entries {
            match entry {
                UiEntry::User { text } => lines.push(format!("user: {text}")),
                UiEntry::Assistant { text } => lines.push(format!("assistant: {text}")),
                UiEntry::Thinking { text, .. } => lines.push(format!("thinking: {text}")),
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
                UiEntry::Ask {
                    summary,
                    phase,
                    detail_lines,
                    ..
                } => {
                    lines.push(format!("ask[{phase:?}]: {summary}"));
                    for line in detail_lines {
                        lines.push(line.clone());
                    }
                }
                UiEntry::Compaction {
                    status,
                    summary,
                    tokens_before,
                    tokens_after,
                    detail,
                } => match status {
                    CompactionBlockStatus::Pending => {
                        lines.push("[compaction]".into());
                        lines.push("Compacting…".into());
                    }
                    CompactionBlockStatus::Complete => {
                        lines.push("[compaction]".into());
                        match tokens_after {
                            Some(m) => lines.push(format!(
                                "Compacted from {tokens_before} → {} tokens",
                                crate::widgets::format_token_count(*m)
                            )),
                            None => lines.push(format!("Compacted from {tokens_before} tokens")),
                        }
                        if !summary.is_empty() {
                            lines.push(summary.clone());
                        }
                    }
                    CompactionBlockStatus::Aborted | CompactionBlockStatus::Failed => {
                        lines.push("[compaction]".into());
                        lines.push(
                            detail
                                .clone()
                                .unwrap_or_else(|| "compaction aborted".into()),
                        );
                    }
                },
                UiEntry::ScrollNotice { text } => lines.push(format!("scroll_notice: {text}")),
                UiEntry::Error { text } => lines.push(format!("error: {text}")),
            }
        }
        for (kind, text) in self.streaming_scrollback_tails() {
            lines.push(format!("{}: {text}…", kind.as_str()));
        }
        lines
    }

    pub(crate) fn streaming_scrollback_tails(&self) -> Vec<(StreamingTailKind, &str)> {
        let mut out = Vec::new();
        if self.streaming_think_id.is_some() && !self.streaming_thinking.is_empty() {
            out.push((
                StreamingTailKind::Thinking,
                self.streaming_thinking.as_str(),
            ));
        }
        if !self.streaming_assistant.is_empty() {
            out.push((
                StreamingTailKind::Assistant,
                self.streaming_assistant.as_str(),
            ));
        }
        out
    }

    pub fn note_user_abort(&mut self) {
        self.flush_streaming();
        if !trailing_aborted_note(&self.entries) {
            self.entries.push(UiEntry::ScrollNotice {
                text: "Operation aborted".into(),
            });
        }
        self.clear_streaming_buffers();
        if self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        }
    }

    pub fn clear_streaming_buffers(&mut self) {
        self.streaming_thinking.clear();
        self.streaming_think_id = None;
        self.streaming_assistant.clear();
        self.thought_clock.reset();
        self.current_role = None;
    }

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
            self.entries.push(UiEntry::ScrollNotice {
                text: "(cancelled)".into(),
            });
        }
        if self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        }
    }

    pub fn begin_bash_block(&mut self, command: &str, exclude_from_context: bool) {
        let ordinal = self
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::Bash { .. }))
            .count();
        self.entries.push(UiEntry::Bash {
            id: format!("bash-live-{ordinal}"),
            command: command.to_string(),
            status: BashBlockStatus::Pending,
            output: String::new(),
            exclude_from_context,
        });
        self.bash_utf8_pending.clear();
        self.phase = UiPhase::Busy;
        self.status = Some("Running".into());
    }

    pub fn bash_block_id_from_command(prior: &[UiEntry], command: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        command.hash(&mut h);
        let ordinal = prior
            .iter()
            .filter(|e| matches!(e, UiEntry::Bash { .. }))
            .count();
        format!("bash-cmd-{:016x}-{ordinal}", h.finish())
    }

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

    pub fn on_stream_closed_without_agent_end(&mut self) {
        if self.phase == UiPhase::Busy && self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        }
    }

    pub(crate) fn flush_streaming(&mut self) {
        self.flush_streaming_elapsed(None);
    }

    pub(crate) fn flush_streaming_elapsed(&mut self, elapsed_override: Option<u64>) {
        self.flush_thinking_elapsed(elapsed_override);
        if !self.streaming_assistant.is_empty() {
            self.entries.push(UiEntry::Assistant {
                text: std::mem::take(&mut self.streaming_assistant),
            });
        }
    }

    pub(crate) fn flush_thinking_elapsed(&mut self, elapsed_override: Option<u64>) {
        if self.streaming_thinking.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.streaming_thinking);
        self.streaming_think_id = None;
        let elapsed_secs = match elapsed_override {
            Some(secs) => (secs > 0).then_some(secs),
            None => {
                if !self.thought_clock.has_ended() {
                    self.thought_clock.stamp_end();
                }
                self.thought_clock.elapsed_secs()
            }
        };
        self.thought_clock.reset();
        let id = allocate_thinking_id(&self.entries, &text);
        self.entries.push(UiEntry::Thinking {
            id,
            text,
            elapsed_secs,
        });
    }

    pub fn set_busy_status(&mut self, status: impl Into<String>) {
        self.phase = UiPhase::Busy;
        self.status = Some(status.into());
    }

    pub(crate) fn maybe_idle_after_agent_end(&mut self) {
        if self.queue.follow_up_count == 0 {
            self.phase = UiPhase::Idle;
            self.status = None;
        } else {
            self.set_busy_status("Follow-up pending");
        }
    }
}
