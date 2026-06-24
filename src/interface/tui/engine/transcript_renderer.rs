//! Transcript rendering to ANSI-styled lines.
//!
//! Converts the pure `Transcript` state into `Vec<String>` lines,
//! where each line may contain ANSI escape sequences for styling.
//!
//! This replaces `render/transcript.rs` (ratatui widgets) for the new engine.

use crate::interface::tui::engine::ansi;
use crate::interface::tui::state::{ToolStatus, Transcript, TranscriptEntry};

/// Render a Transcript into ANSI-styled lines fitting `max_width`.
///
/// Each `TranscriptEntry` produces one or more lines.
/// Entry boundaries are separated by an empty line for readability.
///
/// `scroll_offset` controls scrollback. 0 = show all entries (latest at bottom).
/// N = skip the last N entries so older content comes into view.
pub(crate) fn render_transcript(
    transcript: &Transcript,
    max_width: u16,
    scroll_offset: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    // Apply scroll offset: render only entries up to `entries.len() - scroll_offset`.
    // scroll_offset=0 → all entries; scroll_offset=N → exclude last N entries.
    let visible_count = transcript.entries.len().saturating_sub(scroll_offset);
    let visible_entries = &transcript.entries[..visible_count];

    for entry in visible_entries {
        match entry {
            TranscriptEntry::User { text } => {
                let prefix = ansi::user("▶ ");
                for line_text in text.lines() {
                    let wrapped = ansi::wrap_text(line_text, max_width.saturating_sub(4));
                    for w in wrapped {
                        let styled = format!("  {prefix}{}", ansi::user(&w));
                        lines.push(ansi::pad_to_width(&styled, max_width));
                    }
                }
            }
            TranscriptEntry::Assistant { text, streaming } => {
                let prefix_str = if *streaming { "▸ " } else { "  " };
                let prefix = ansi::assistant(prefix_str);
                for line_text in text.lines() {
                    let wrapped = ansi::wrap_text(line_text, max_width.saturating_sub(4));
                    for w in wrapped {
                        let styled = format!("  {prefix}{}", ansi::assistant(&w));
                        lines.push(ansi::pad_to_width(&styled, max_width));
                    }
                }
            }
            TranscriptEntry::ToolCall {
                name,
                status,
                output,
                ..
            } => {
                let (icon, style_fn): (&str, fn(&str) -> String) = match status {
                    ToolStatus::Running => ("● ", ansi::hint),
                    ToolStatus::Success => ("✓ ", ansi::success),
                    ToolStatus::Failed => ("✗ ", ansi::error),
                };
                let tool_part = ansi::tool(&format!("[{name}]"));
                let line = format!("  {}{}{}", style_fn(icon), tool_part, ansi::reset());
                lines.push(ansi::pad_to_width(&line, max_width));

                // Show first line of output if present
                if let Some(first_line) = output.lines().next() {
                    let truncated = if first_line.len() > max_width.saturating_sub(6) as usize {
                        format!("{}...", &first_line[..max_width.saturating_sub(9) as usize])
                    } else {
                        first_line.to_string()
                    };
                    let out_line = format!("    {}", ansi::dim_text(&truncated));
                    lines.push(ansi::pad_to_width(&out_line, max_width));
                }
            }
            TranscriptEntry::Compaction { reason } => {
                let line = format!(
                    "  {} {}",
                    ansi::hint("⚡"),
                    ansi::dim_text(&format!("[compaction: {reason}]"))
                );
                lines.push(ansi::pad_to_width(&line, max_width));
            }
            TranscriptEntry::Error(msg) => {
                let line = format!("  {}", ansi::error(msg));
                lines.push(ansi::pad_to_width(&line, max_width));
            }
            TranscriptEntry::Info(msg) => {
                let line = format!("  {}", ansi::hint(msg));
                lines.push(ansi::pad_to_width(&line, max_width));
            }
        }

        // Blank line between entries
        lines.push(String::new());
    }

    // If empty, show placeholder
    if lines.is_empty() || lines.iter().all(|l| l.trim().is_empty()) {
        let placeholder = ansi::dim_text("  No messages yet. Type a message to start.");
        lines.push(ansi::pad_to_width(&placeholder, max_width));
    }

    lines
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
        assert!(lines[0].contains("No messages yet"));
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
        assert!(lines.iter().any(|l| l.contains("▶")));
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
        assert!(lines.iter().any(|l| l.contains("▸")));
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
        assert!(lines.iter().any(|l| l.contains("[bash]")));
    }

    #[test]
    fn test_error_entry() {
        let t = make_transcript(vec![AgentEvent::Error("oops".into())]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("oops")));
    }

    #[test]
    fn test_compaction() {
        let t = make_transcript(vec![AgentEvent::CompactionStart {
            reason: "window full".into(),
        }]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("compaction")));
    }

    #[test]
    fn test_model_select() {
        let t = make_transcript(vec![AgentEvent::ModelSelect {
            provider: "openai".into(),
            model_id: "gpt-4".into(),
        }]);
        let lines = render_transcript(&t, 80, 0);
        assert!(lines.iter().any(|l| l.contains("gpt-4")));
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
}
