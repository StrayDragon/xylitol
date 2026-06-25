//! Transcript rendering to ANSI-styled lines.
//!
//! Converts the pure `Transcript` state into `Vec<String>` lines,
//! where each line may contain ANSI escape sequences for styling.
//!
//! The transcript grows UPWARD from the input area: the newest entries
//! appear at the bottom of the transcript (closest to the input separator).
//! Older entries scroll off the top of the screen.
//!
//! Pure characters only — no emoji. User messages use `>` prefix.

use crate::interface::tui::engine::ansi;

use crate::interface::tui::state::{ToolStatus, Transcript, TranscriptEntry};

/// Render a Transcript into ANSI-styled lines that grow upward from the bottom.
///
/// Returns all lines for the visible entries (oldest first).
/// The caller is expected to take only the LAST `max_lines` lines when
/// displaying in a fixed-height transcript area.
///
/// `scroll_offset` controls scrollback. 0 = show all entries.
/// N = skip the last N entries so older content comes into view.
pub(crate) fn render_transcript(
    transcript: &Transcript,
    max_width: u16,
    scroll_offset: usize,
) -> Vec<String> {
    fn wrap_indent(text: &str, prefix: &str, indent: &str, max_width: u16) -> Vec<String> {
        let mut lines = Vec::new();
        for (i, src_line) in text.lines().enumerate() {
            let pfx = if i == 0 { prefix } else { indent };
            let available = max_width.saturating_sub(ansi::visible_width(pfx) as u16);
            let wrapped = ansi::wrap_text(src_line, available);
            for w in wrapped {
                let styled = format!("{pfx}{}", ansi::assistant(&w));
                lines.push(ansi::pad_to_width(&styled, max_width));
            }
        }
        if text.is_empty() {
            let styled = format!("{prefix}");
            lines.push(ansi::pad_to_width(&styled, max_width));
        }
        lines
    }

    // Apply scroll offset: render only entries up to `entries.len() - scroll_offset`.
    let visible_count = transcript.entries.len().saturating_sub(scroll_offset);
    let visible_entries = &transcript.entries[..visible_count];

    let mut lines: Vec<String> = Vec::new();

    for entry in visible_entries {
        match entry {
            TranscriptEntry::User { text } => {
                // Single line: "> text"
                for (i, src_line) in text.lines().enumerate() {
                    let pfx = if i == 0 {
                        ansi::user("> ")
                    } else {
                        "  ".to_string()
                    };
                    let available = max_width.saturating_sub(2);
                    let wrapped = ansi::wrap_text(src_line, available);
                    for w in wrapped {
                        let styled = format!("{pfx}{}", ansi::user(&w));
                        lines.push(ansi::pad_to_width(&styled, max_width));
                    }
                }
                // Blank line after user message for readability
                lines.push(String::new());
            }
            TranscriptEntry::Assistant { text, .. } => {
                let wrapped = wrap_indent(text, "  ", "  ", max_width);
                lines.extend(wrapped);
                lines.push(String::new());
            }
            TranscriptEntry::ToolCall {
                name,
                status,
                output,
                ..
            } => {
                let (label, style_fn): (&str, fn(&str) -> String) = match status {
                    ToolStatus::Running => ("running", ansi::hint),
                    ToolStatus::Success => ("ok", ansi::success),
                    ToolStatus::Failed => ("FAIL", ansi::error),
                };
                // "[tool:name] status"
                let tool_part = ansi::tool(&format!("[tool:{name}]"));
                let status_part = style_fn(&format!(" {label}"));
                let line = format!("  {tool_part}{status_part}");
                lines.push(ansi::pad_to_width(&line, max_width));

                // First line of output (if any)
                if let Some(first_line) = output.lines().next()
                    && !first_line.trim().is_empty()
                {
                    let truncated = if first_line.len() > max_width.saturating_sub(6) as usize {
                        format!("{}...", &first_line[..max_width.saturating_sub(9) as usize])
                    } else {
                        first_line.to_string()
                    };
                    let out_line = format!("    {}", ansi::dim_text(&truncated));
                    lines.push(ansi::pad_to_width(&out_line, max_width));
                }
                lines.push(String::new());
            }
            TranscriptEntry::Compaction { reason } => {
                let line = format!("  {}", ansi::dim_text(&format!("[compact: {reason}]")));
                lines.push(ansi::pad_to_width(&line, max_width));
                lines.push(String::new());
            }
            TranscriptEntry::Error(msg) => {
                let line = format!("  {}", ansi::error(&format!("[error: {msg}]")));
                lines.push(ansi::pad_to_width(&line, max_width));
                lines.push(String::new());
            }
            TranscriptEntry::Info(msg) => {
                let line = format!("  {}", ansi::dim_text(&format!("[info: {msg}]")));
                lines.push(ansi::pad_to_width(&line, max_width));
                lines.push(String::new());
            }
        }
    }

    // If empty, show placeholder
    if lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
        let placeholder = "  No messages yet.";
        let truncated = ansi::truncate_visible(placeholder, max_width.saturating_sub(1) as usize);
        let styled = ansi::dim_text(&truncated);
        lines.push(ansi::pad_to_width(&styled, max_width));
        lines.push(String::new());
    }

    lines
}

/// Take only the last `max_lines` lines from a rendered transcript.
/// This implements the "newest at bottom" layout — the oldest lines
/// scroll off the top when the transcript is longer than available space.
pub(crate) fn trim_to_viewport(lines: Vec<String>, max_lines: usize) -> Vec<String> {
    let len = lines.len();
    if len <= max_lines {
        return lines;
    }
    lines[len.saturating_sub(max_lines)..].to_vec()
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::r#loop::AgentEvent;
    use crate::interface::tui::state::Transcript;

    fn make_transcript(events: Vec<AgentEvent>) -> Transcript {
        let mut t = Transcript::new();
        for e in events {
            t.apply(e);
        }
        t
    }

    #[test]
    fn test_empty() {
        let t = Transcript::new();
        let lines = render_transcript(&t, 80, 0);
        assert!(!lines.is_empty());
        assert!(lines.iter().any(|l| l.contains("No messages yet")));
    }

    #[test]
    fn test_user_message() {
        let t = make_transcript(vec![
            AgentEvent::MessageStart {
                role: "user".into(),
            },
            AgentEvent::TextDelta("Hello!".into()),
            AgentEvent::MessageEnd {
                role: "user".into(),
            },
        ]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("Hello!")));
        assert!(lines.iter().any(|l| l.contains(">")));
        // No emoji
        assert!(!lines.iter().any(|l| l.contains('▶')));
    }

    #[test]
    fn test_assistant_streaming() {
        let t = make_transcript(vec![
            AgentEvent::TurnStart { turn_index: 1 },
            AgentEvent::MessageStart {
                role: "assistant".into(),
            },
            AgentEvent::TextDelta("Hello from AI".into()),
        ]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("Hello from AI")));
    }

    #[test]
    fn test_tool_call() {
        let t = make_transcript(vec![
            AgentEvent::ToolExecutionStart {
                id: "t1".into(),
                name: "bash".into(),
                args: serde_json::json!({"cmd": "ls"}),
            },
            AgentEvent::ToolExecutionEnd {
                id: "t1".into(),
                name: "bash".into(),
                result: "file.txt".into(),
            },
        ]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("[tool:bash]")));
        assert!(lines.iter().any(|l| l.contains("ok")));
        // No emoji
        assert!(!lines.iter().any(|l| l.contains('✓')));
        assert!(!lines.iter().any(|l| l.contains('●')));
    }

    #[test]
    fn test_error_entry() {
        let t = make_transcript(vec![AgentEvent::Error("oops".into())]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("oops")));
        assert!(lines.iter().any(|l| l.contains("[error:")));
    }

    #[test]
    fn test_compaction() {
        let t = make_transcript(vec![AgentEvent::CompactionStart {
            reason: "window full".into(),
        }]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("compact")));
        // No emoji
        assert!(!lines.iter().any(|l| l.contains('⚡')));
    }

    #[test]
    fn test_model_select() {
        let t = make_transcript(vec![AgentEvent::ModelSelect {
            provider: "openai".into(),
            model_id: "gpt-4".into(),
        }]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("gpt-4")));
        assert!(lines.iter().any(|l| l.contains("[info:")));
    }

    #[test]
    fn test_ansi_stripped_width() {
        let t = make_transcript(vec![
            AgentEvent::TurnStart { turn_index: 1 },
            AgentEvent::MessageStart {
                role: "assistant".into(),
            },
            AgentEvent::TextDelta("hi".into()),
        ]);
        let lines = render_transcript(&t, 80, 0);
        for line in &lines {
            let vw = ansi::visible_width(line);
            assert!(
                vw <= 80,
                "line width {vw} exceeds 80: {:?}",
                ansi::strip_ansi(line)
            );
        }
    }

    #[test]
    fn test_scroll_offset_hides_last_entry() {
        let t = make_transcript(vec![
            AgentEvent::MessageStart {
                role: "user".into(),
            },
            AgentEvent::TextDelta("first".into()),
            AgentEvent::MessageEnd {
                role: "user".into(),
            },
            AgentEvent::MessageStart {
                role: "user".into(),
            },
            AgentEvent::TextDelta("second".into()),
            AgentEvent::MessageEnd {
                role: "user".into(),
            },
        ]);
        let lines = render_transcript(&t, 80, 1);
        assert!(lines.iter().any(|l| l.contains("first")));
        assert!(!lines.iter().any(|l| l.contains("second")));
    }

    #[test]
    fn test_scroll_offset_zero_shows_all() {
        let t = make_transcript(vec![
            AgentEvent::MessageStart {
                role: "user".into(),
            },
            AgentEvent::TextDelta("alpha".into()),
            AgentEvent::MessageEnd {
                role: "user".into(),
            },
            AgentEvent::MessageStart {
                role: "user".into(),
            },
            AgentEvent::TextDelta("beta".into()),
            AgentEvent::MessageEnd {
                role: "user".into(),
            },
        ]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("alpha")));
        assert!(lines.iter().any(|l| l.contains("beta")));
    }

    #[test]
    fn test_trim_to_viewport_keeps_last_n() {
        let lines: Vec<String> = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        let trimmed = trim_to_viewport(lines, 2);
        assert_eq!(trimmed.len(), 2);
        assert_eq!(trimmed[0], "c");
        assert_eq!(trimmed[1], "d");
    }

    #[test]
    fn test_trim_to_viewport_shorter_than_max() {
        let lines: Vec<String> = vec!["a".into(), "b".into()];
        let trimmed = trim_to_viewport(lines, 5);
        assert_eq!(trimmed.len(), 2);
    }

    #[test]
    fn test_failed_tool_call() {
        let t = make_transcript(vec![
            AgentEvent::ToolExecutionStart {
                id: "t1".into(),
                name: "bash".into(),
                args: serde_json::json!({"cmd": "ls"}),
            },
            AgentEvent::ToolExecutionEnd {
                id: "t1".into(),
                name: "bash".into(),
                result: "error: not found".into(),
            },
        ]);
        // Manually set to failed
        let mut t2 = t.clone();
        if let Some(entry) = t2.entries.last_mut() {
            if let TranscriptEntry::ToolCall { status, .. } = entry {
                *status = ToolStatus::Failed;
            }
        }
        let lines = render_transcript(&t2, 80, 0);
        assert!(lines.iter().any(|l| l.contains("FAIL")));
    }

    #[test]
    fn test_running_tool_call() {
        let t = make_transcript(vec![AgentEvent::ToolExecutionStart {
            id: "t1".into(),
            name: "read".into(),
            args: serde_json::json!({"path": "file.rs"}),
        }]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("running")));
    }
}
