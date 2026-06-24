//! Transcript state machine — consumes AgentEvent and produces rendered entries.
//!
//! This module is a PURE FUNCTION state machine. It does NOT depend on ratatui
//! or any TUI crate — only on `crate::agent::loop::AgentEvent`.

use crate::agent::r#loop::AgentEvent;

/// A single entry in the transcript.
#[derive(Debug, Clone)]
pub(crate) enum TranscriptEntry {
    /// User message (text + optional images).
    User { text: String },
    /// Assistant message (streaming text).
    Assistant {
        text: String,
        /// Whether this entry is still being streamed.
        streaming: bool,
    },
    /// Tool execution.
    ToolCall {
        id: String,
        name: String,
        status: ToolStatus,
        output: String,
    },
    /// Compaction notice.
    Compaction { reason: String },
    /// Error message.
    Error(String),
    /// System event (model switch, etc.)
    Info(String),
}

/// Status of a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolStatus {
    Running,
    Success,
    Failed,
}

/// Transcript — the core state machine for conversation history.
#[derive(Debug, Clone)]
pub(crate) struct Transcript {
    /// All entries in chronological order.
    pub(crate) entries: Vec<TranscriptEntry>,
    /// Index of the entry currently being streamed (if any).
    pub(crate) streaming_idx: Option<usize>,
    /// Total turn count.
    pub(crate) turn_count: u32,
    /// Number of entries scrolled back from the bottom. 0 = latest.
    pub(crate) scroll_offset: usize,
}

impl Transcript {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            streaming_idx: None,
            turn_count: 0,
            scroll_offset: 0,
        }
    }

    /// Apply an AgentEvent and update the transcript state.
    ///
    /// Pure function — no I/O, no TUI dependencies.
    /// Automatically resets scroll offset when new content arrives.
    pub(crate) fn apply(&mut self, event: AgentEvent) {
        // Reset scroll on new content so user sees latest messages
        let reset_scroll = matches!(
            event,
            AgentEvent::MessageStart { .. }
                | AgentEvent::TextDelta(_)
                | AgentEvent::ToolExecutionStart { .. }
                | AgentEvent::Error(_)
        );
        if reset_scroll {
            self.scroll_offset = 0;
        }
        match event {
            AgentEvent::TurnStart { .. } => {
                self.turn_count += 1;
            }
            AgentEvent::MessageStart { role } => match role.as_str() {
                "assistant" | "model" => {
                    let idx = self.entries.len();
                    self.entries.push(TranscriptEntry::Assistant {
                        text: String::new(),
                        streaming: true,
                    });
                    self.streaming_idx = Some(idx);
                }
                "user" => {
                    let idx = self.entries.len();
                    self.entries.push(TranscriptEntry::User {
                        text: String::new(),
                    });
                    self.streaming_idx = Some(idx);
                }
                _ => {}
            },
            AgentEvent::TextDelta(delta) => {
                if let Some(idx) = self.streaming_idx {
                    if let Some(entry) = self.entries.get_mut(idx) {
                        match entry {
                            TranscriptEntry::Assistant { text, .. }
                            | TranscriptEntry::User { text } => {
                                text.push_str(&delta);
                            }
                            _ => {}
                        }
                    }
                } else if !self.entries.is_empty() {
                    // Fallback: append to the last entry if it's Assistant or User
                    if let Some(entry) = self.entries.last_mut() {
                        match entry {
                            TranscriptEntry::Assistant { text, .. }
                            | TranscriptEntry::User { text } => {
                                text.push_str(&delta);
                            }
                            _ => {}
                        }
                    }
                }
            }
            AgentEvent::ThinkingDelta(_) => {
                // P0: silently discard thinking deltas
            }
            AgentEvent::MessageUpdate { text, .. } => {
                if let Some(idx) = self.streaming_idx {
                    if let Some(entry) = self.entries.get_mut(idx) {
                        if let TranscriptEntry::Assistant { text: content, .. } = entry {
                            *content = text;
                        }
                    }
                }
            }
            AgentEvent::MessageEnd { .. } => {
                if let Some(idx) = self.streaming_idx {
                    if let Some(entry) = self.entries.get_mut(idx) {
                        if let TranscriptEntry::Assistant { streaming, .. } = entry {
                            *streaming = false;
                        }
                    }
                    self.streaming_idx = None;
                }
            }
            AgentEvent::ToolExecutionStart { id, name, .. } => {
                self.entries.push(TranscriptEntry::ToolCall {
                    id,
                    name,
                    status: ToolStatus::Running,
                    output: String::new(),
                });
            }
            AgentEvent::ToolExecutionUpdate { id, output } => {
                let new_output = output;
                for entry in self.entries.iter_mut().rev() {
                    if let TranscriptEntry::ToolCall {
                        id: eid, output, ..
                    } = entry
                    {
                        if eid == &id {
                            output.push_str(&new_output);
                            break;
                        }
                    }
                }
            }
            AgentEvent::ToolExecutionEnd {
                id,
                name: _,
                result,
            } => {
                let tool_result = result;
                for entry in self.entries.iter_mut().rev() {
                    if let TranscriptEntry::ToolCall {
                        id: eid,
                        status,
                        output,
                        ..
                    } = entry
                    {
                        if eid == &id {
                            *status = ToolStatus::Success;
                            if !tool_result.is_empty() {
                                output.push_str("Result: ");
                                output.push_str(&tool_result);
                            }
                            break;
                        }
                    }
                }
            }
            AgentEvent::TurnEnd { .. } => {
                // No specific rendering needed
            }
            AgentEvent::Error(msg) => {
                self.entries.push(TranscriptEntry::Error(msg));
                self.streaming_idx = None;
            }
            AgentEvent::AgentEnd { .. } => {
                self.streaming_idx = None;
            }
            AgentEvent::CompactionStart { reason } => {
                self.entries.push(TranscriptEntry::Compaction { reason });
            }
            AgentEvent::CompactionEnd { .. } => {
                // No specific rendering needed
            }
            AgentEvent::ModelSelect { provider, model_id } => {
                self.entries.push(TranscriptEntry::Info(format!(
                    "Switched to {model_id} ({provider})"
                )));
            }
            AgentEvent::ThinkingLevelChanged { level } => {
                self.entries
                    .push(TranscriptEntry::Info(format!("Thinking level: {level}")));
            }
        }
    }

    /// Check if the agent is currently processing (streaming or tool running).
    pub(crate) fn is_running(&self) -> bool {
        self.streaming_idx.is_some()
            || self.entries.iter().any(|e| {
                matches!(
                    e,
                    TranscriptEntry::ToolCall {
                        status: ToolStatus::Running,
                        ..
                    }
                )
            })
    }

    /// Scroll up (back in history) by `n` entries.
    pub(crate) fn scroll_up(&mut self, n: usize) {
        let max_offset = self.entries.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + n).min(max_offset);
    }

    /// Scroll down (towards latest) by `n` entries.
    pub(crate) fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Reset scroll to latest.
    pub(crate) fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
    }

    /// Get the last user message text, if any.
    pub(crate) fn last_user_message(&self) -> Option<&str> {
        self.entries.iter().rev().find_map(|e| {
            if let TranscriptEntry::User { text } = e {
                Some(text.as_str())
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_event(s: &str) -> AgentEvent {
        AgentEvent::TextDelta(s.to_string())
    }

    #[test]
    fn test_new_transcript_is_empty() {
        let t = Transcript::new();
        assert!(t.entries.is_empty());
        assert_eq!(t.turn_count, 0);
    }

    #[test]
    fn test_message_start_creates_entry() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "assistant".into(),
        });
        assert_eq!(t.entries.len(), 1);
        assert!(matches!(t.entries[0], TranscriptEntry::Assistant { .. }));
        assert!(t.streaming_idx.is_some());
    }

    #[test]
    fn test_text_delta_appends() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "assistant".into(),
        });
        t.apply(text_event("Hello"));
        t.apply(text_event(" world"));
        if let TranscriptEntry::Assistant { text, .. } = &t.entries[0] {
            assert_eq!(text, "Hello world");
        } else {
            panic!("expected Assistant entry");
        }
    }

    #[test]
    fn test_message_end_stops_streaming() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "assistant".into(),
        });
        t.apply(text_event("Hi"));
        t.apply(AgentEvent::MessageEnd {
            role: "assistant".into(),
        });
        assert!(t.streaming_idx.is_none());
        if let TranscriptEntry::Assistant { streaming, .. } = &t.entries[0] {
            assert!(!streaming);
        } else {
            panic!("expected Assistant entry");
        }
    }

    #[test]
    fn test_tool_execution_creates_entry() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::ToolExecutionStart {
            id: "call-1".into(),
            name: "bash".into(),
            args: serde_json::json!({"command": "ls"}),
        });
        assert_eq!(t.entries.len(), 1);
        assert!(matches!(
            t.entries[0],
            TranscriptEntry::ToolCall {
                status: ToolStatus::Running,
                ..
            }
        ));
    }

    #[test]
    fn test_tool_execution_end() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::ToolExecutionStart {
            id: "call-1".into(),
            name: "bash".into(),
            args: serde_json::json!({"command": "ls"}),
        });
        t.apply(AgentEvent::ToolExecutionEnd {
            id: "call-1".into(),
            name: "bash".into(),
            result: "file1\nfile2".into(),
        });
        if let TranscriptEntry::ToolCall { status, .. } = &t.entries[0] {
            assert_eq!(*status, ToolStatus::Success);
        } else {
            panic!("expected ToolCall entry");
        }
    }

    #[test]
    fn test_error_entry() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::Error("something went wrong".into()));
        assert_eq!(t.entries.len(), 1);
        assert!(matches!(t.entries[0], TranscriptEntry::Error(_)));
    }

    #[test]
    fn test_compaction_entry() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::CompactionStart {
            reason: "token limit".into(),
        });
        assert!(matches!(t.entries[0], TranscriptEntry::Compaction { .. }));
    }

    #[test]
    fn test_is_running_with_streaming() {
        let mut t = Transcript::new();
        assert!(!t.is_running());
        t.apply(AgentEvent::MessageStart {
            role: "assistant".into(),
        });
        assert!(t.is_running());
        t.apply(AgentEvent::MessageEnd {
            role: "assistant".into(),
        });
        assert!(!t.is_running());
    }

    #[test]
    fn test_is_running_with_tool() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::ToolExecutionStart {
            id: "t1".into(),
            name: "bash".into(),
            args: serde_json::json!({"cmd": "ls"}),
        });
        assert!(t.is_running());
    }

    #[test]
    fn test_model_select_info() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::ModelSelect {
            provider: "openai".into(),
            model_id: "gpt-4".into(),
        });
        assert!(matches!(t.entries[0], TranscriptEntry::Info(_)));
    }

    #[test]
    fn test_turn_count() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::TurnStart { turn_index: 0 });
        assert_eq!(t.turn_count, 1);
        t.apply(AgentEvent::TurnStart { turn_index: 1 });
        assert_eq!(t.turn_count, 2);
    }

    #[test]
    fn test_full_conversation_flow() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::TurnStart { turn_index: 0 });
        t.apply(AgentEvent::MessageStart {
            role: "assistant".into(),
        });
        t.apply(text_event("Hello! How can I help?"));
        t.apply(AgentEvent::MessageEnd {
            role: "assistant".into(),
        });
        assert_eq!(t.entries.len(), 1);
        assert!(!t.is_running());
    }

    #[test]
    fn test_user_message_creates_entry() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "user".into(),
        });
        assert_eq!(t.entries.len(), 1);
        assert!(matches!(t.entries[0], TranscriptEntry::User { .. }));
        assert!(t.streaming_idx.is_some());
    }

    #[test]
    fn test_user_message_text_delta() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "user".into(),
        });
        t.apply(text_event("Hello, world"));
        assert_eq!(t.entries.len(), 1);
        if let TranscriptEntry::User { text } = &t.entries[0] {
            assert_eq!(text, "Hello, world");
        } else {
            panic!("expected User entry");
        }
    }

    #[test]
    fn test_user_message_end_clears_streaming() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "user".into(),
        });
        assert!(t.streaming_idx.is_some());
        t.apply(AgentEvent::MessageEnd {
            role: "user".into(),
        });
        assert!(t.streaming_idx.is_none());
    }

    #[test]
    fn test_user_and_assistant_ordering() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "user".into(),
        });
        t.apply(text_event("What is Rust?"));
        t.apply(AgentEvent::MessageEnd {
            role: "user".into(),
        });
        t.apply(AgentEvent::MessageStart {
            role: "assistant".into(),
        });
        t.apply(text_event("Rust is a systems language."));
        t.apply(AgentEvent::MessageEnd {
            role: "assistant".into(),
        });
        assert_eq!(t.entries.len(), 2);
        assert!(matches!(t.entries[0], TranscriptEntry::User { .. }));
        assert!(matches!(t.entries[1], TranscriptEntry::Assistant { .. }));
        if let TranscriptEntry::User { text } = &t.entries[0] {
            assert_eq!(text, "What is Rust?");
        }
    }

    // ── Scroll tests ──

    #[test]
    fn test_scroll_offset_starts_at_zero() {
        let t = Transcript::new();
        assert_eq!(t.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_up() {
        let mut t = Transcript::new();
        // Add some entries
        for i in 0..5 {
            t.apply(AgentEvent::MessageStart {
                role: "user".into(),
            });
            t.apply(text_event(&format!("msg {i}")));
            t.apply(AgentEvent::MessageEnd {
                role: "user".into(),
            });
            t.apply(AgentEvent::MessageStart {
                role: "assistant".into(),
            });
            t.apply(text_event(&format!("reply {i}")));
            t.apply(AgentEvent::MessageEnd {
                role: "assistant".into(),
            });
        }
        t.scroll_up(2);
        assert_eq!(t.scroll_offset, 2);
    }

    #[test]
    fn test_scroll_down() {
        let mut t = Transcript::new();
        for i in 0..5 {
            t.apply(AgentEvent::MessageStart {
                role: "user".into(),
            });
            t.apply(text_event(&format!("msg {i}")));
            t.apply(AgentEvent::MessageEnd {
                role: "user".into(),
            });
        }
        t.scroll_up(3);
        assert_eq!(t.scroll_offset, 3);
        t.scroll_down(1);
        assert_eq!(t.scroll_offset, 2);
        t.scroll_down(5);
        assert_eq!(t.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_up_caps_at_max() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "user".into(),
        });
        t.apply(text_event("hi"));
        t.apply(AgentEvent::MessageEnd {
            role: "user".into(),
        });
        // Only 1 entry, max offset = 0
        t.scroll_up(10);
        assert_eq!(t.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_reset_on_new_content() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "user".into(),
        });
        t.apply(text_event("msg 1"));
        t.apply(AgentEvent::MessageEnd {
            role: "user".into(),
        });
        t.scroll_up(1);
        // New MessageStart should reset scroll
        t.apply(AgentEvent::MessageStart {
            role: "assistant".into(),
        });
        assert_eq!(t.scroll_offset, 0);
    }

    #[test]
    fn test_reset_scroll() {
        let mut t = Transcript::new();
        t.apply(AgentEvent::MessageStart {
            role: "user".into(),
        });
        t.apply(text_event("msg 1"));
        t.apply(AgentEvent::MessageEnd {
            role: "user".into(),
        });
        t.scroll_up(1);
        t.reset_scroll();
        assert_eq!(t.scroll_offset, 0);
    }
}
