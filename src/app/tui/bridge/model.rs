//! UI-only model types and methods (c1170).

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
        Some(UiEntry::ScrollNotice { text })
            if text == "Aborted"
                || text == "aborted"
                || text == "Operation aborted"
                || text.starts_with("Operation aborted")
    )
}

/// True when the last line is already a bang `(cancelled)` note (pi bash status).
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

/// User-visible busy vs idle (idle ⇒ status row is 0 lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPhase {
    Idle,
    Busy,
}

/// Queue badge counts from [`crate::protocol::lifecycle::XyEvent::QueueUpdate`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueBadge {
    pub steer_count: usize,
    pub follow_up_count: usize,
}

/// Bang / interactive bash block tint state (c668; aligns demo tool tint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BashBlockStatus {
    Pending,
    Success,
    Error,
    Cancelled,
}

/// Builtin `ask` scrollback phase (c1850; rail semantic colors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AskPhase {
    Waiting,
    Answered,
    Skipped,
}

/// Transcript compaction block phase (c1730; aligns pi CompactionSummaryMessageComponent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompactionBlockStatus {
    /// CompactionStart placeholder.
    Pending,
    /// CompactionEnd success / resume CompactionEntry.
    Complete,
    /// CompactionEnd aborted.
    Aborted,
    /// CompactionEnd failed (error_message).
    Failed,
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
        /// Sticky path from streaming args / End backfill (c1320).
        tool_path: Option<String>,
        /// Streaming write body (args.content); shown with Head viewport (c1300).
        write_content: Option<String>,
        /// Edit display_diff merged into the same tool row (c1300).
        display_diff: Option<String>,
        output: String,
        is_error: bool,
        done: bool,
    },
    /// Builtin `ask` questionnaire (c1850) — dedicated rail entry, not generic Tool wash.
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
    /// Interactive `!` / `!!` bash block (c668).
    Bash {
        command: String,
        status: BashBlockStatus,
        output: String,
        exclude_from_context: bool,
    },
    /// Context compaction transcript block (c1730).
    Compaction {
        status: CompactionBlockStatus,
        summary: String,
        tokens_before: u64,
        /// Short failure / abort detail when not Complete.
        detail: Option<String>,
    },
    ScrollNotice {
        text: String,
    },
    Error {
        text: String,
    },
}

/// Product TUI state produced solely by [`super::apply_xy_event`] / [`UiModel::begin_run`].
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

    /// Enqueue a steer message for strip + badge (host local; XyDriver follows).
    pub fn enqueue_steer_strip(&mut self, text: String) {
        self.pending_steer.push(text);
        self.queue.steer_count = self.pending_steer.len();
    }

    /// Enqueue a follow-up message for strip + badge (host local; XyDriver follows).
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
                    detail,
                } => match status {
                    CompactionBlockStatus::Pending => {
                        lines.push("[compaction]".into());
                        lines.push("Compacting…".into());
                    }
                    CompactionBlockStatus::Complete => {
                        lines.push("[compaction]".into());
                        lines.push(format!("Compacted from {tokens_before} tokens"));
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

    /// User Esc/Ctrl+C abort: flush streamed partial into scrollback + abort footer (c1595 / pi).
    /// Dedupes only a trailing abort note (Esc repeat / same-event `Error("aborted")`).
    /// Bang Esc uses [`Self::note_bash_cancelled`] instead (pi `(cancelled)`).
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

    /// Drop in-flight stream drafts (c670 / c720 Esc latch before drain).
    pub fn clear_streaming_buffers(&mut self) {
        self.streaming_thinking.clear();
        self.streaming_assistant.clear();
        self.current_role = None;
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
            self.entries.push(UiEntry::ScrollNotice {
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
        // Follow-ups are drained inside the same XyDriver::run before AgentEnd on
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
