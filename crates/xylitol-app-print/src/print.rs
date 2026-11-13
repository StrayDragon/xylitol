use std::io::{self, Write};

use crate::tool_display::{is_mcp_tool_name, pretty_json_text, pretty_json_value};
use futures::StreamExt;
use std::pin::Pin;
use xylitol_runtime::XyEvent;

pub type EventStream = Pin<Box<dyn futures::Stream<Item = XyEvent> + Send>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThinkBlockPhase {
    Outside,
    Inside { model_open: bool, model_close: bool },
}

#[derive(Debug, thiserror::Error)]
pub enum PrintError {
    #[error("{0}")]
    Message(String),
}

pub async fn run_print<S>(stream: &mut S, _session_id: &str) -> Result<(), PrintError>
where
    S: futures::Stream<Item = XyEvent> + Unpin,
{
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let stderr = io::stderr();
    let mut err_handle = stderr.lock();

    render_stream(stream, &mut handle, &mut err_handle).await
}

pub async fn render_stream<S, W: Write, E: Write>(
    stream: &mut S,
    writer: &mut W,
    err: &mut E,
) -> Result<(), PrintError>
where
    S: futures::Stream<Item = XyEvent> + Unpin,
{
    let mut thinking = ThinkBlockPhase::Outside;

    fn close_thinking_block<E: Write>(err: &mut E, thinking: &mut ThinkBlockPhase) {
        if let ThinkBlockPhase::Inside {
            model_open: _,
            model_close: false,
        } = *thinking
        {
            let _ = write!(err, "</think>");
            let _ = err.flush();
        }
        *thinking = ThinkBlockPhase::Outside;
    }

    while let Some(event) = stream.next().await {
        match event {
            XyEvent::StepStart { step_index } => {
                eprintln!("\n[Turn {step_index}]");
            }
            XyEvent::StepEnd { step_index } => {
                eprintln!("\n[Turn {step_index} end]");
            }
            XyEvent::MessageStart { .. } => {
                close_thinking_block(err, &mut thinking);
            }
            XyEvent::MessageEnd { .. } => {
                close_thinking_block(err, &mut thinking);
            }
            XyEvent::TextDelta(text) => {
                close_thinking_block(err, &mut thinking);
                let _ = write!(writer, "{text}");
                let _ = writer.flush();
            }
            XyEvent::ThinkingDelta(text) => {
                match &mut thinking {
                    ThinkBlockPhase::Outside => {
                        let model_open = text.contains("<think>");
                        if !model_open {
                            let _ = write!(err, "<think>");
                        }
                        thinking = ThinkBlockPhase::Inside {
                            model_open,
                            model_close: text.contains("</think>"),
                        };
                    }
                    ThinkBlockPhase::Inside { model_close, .. } => {
                        *model_close |= text.contains("</think>");
                    }
                }
                let _ = write!(err, "{text}");
                let _ = err.flush();
            }
            XyEvent::MessageUpdate { .. } => {}
            XyEvent::ToolExecutionStart { name, args, .. } => {
                for line in format_tool_start_lines(&name, &args) {
                    eprintln!("{line}");
                }
            }
            XyEvent::ToolExecutionUpdate { output, .. } => {
                eprint!("{output}");
                let _ = io::stderr().flush();
            }
            XyEvent::ToolExecutionEnd { name, result, .. } => {
                eprintln!("{}", format_tool_end_line(&name, &result));
            }
            XyEvent::Error(event_err) => {
                let msg = &event_err.message;
                eprintln!("\n[Error] {msg}");
                close_thinking_block(err, &mut thinking);
                return Err(PrintError::Message(msg.clone()));
            }
            XyEvent::CompactionStart { reason } => {
                eprintln!("\n[Compaction] {reason}");
            }
            XyEvent::CompactionEnd { .. } => {
                eprintln!("\n[Compaction complete]");
            }
            XyEvent::ModelSelect { model_id, .. } => {
                eprintln!("\n[Model] switched to {model_id}");
            }
            XyEvent::ThinkingLevelChanged { level } => {
                eprintln!("\n[Thinking] level set to {level}");
            }
            XyEvent::AgentStart { .. }
            | XyEvent::QueueUpdate { .. }
            | XyEvent::AutoRetryStart { .. }
            | XyEvent::AutoRetryEnd { .. }
            | XyEvent::SessionInfoChanged { .. }
            | XyEvent::ContextTokenSettlement { .. }
            | XyEvent::TodoUpdated { .. }
            | XyEvent::WorkflowUpdated { .. } => {}
            XyEvent::AgentEnd { reason, .. } => {
                if let Some(reason) = reason {
                    eprintln!("\n[End] {reason}");
                }
            }
        }
    }
    close_thinking_block(err, &mut thinking);
    let _ = writeln!(writer);
    Ok(())
}

pub fn format_tool_start_lines(name: &str, args: &serde_json::Value) -> Vec<String> {
    let mut lines = vec![format!("\n[Tool: {name}]")];
    if is_mcp_tool_name(name) && !args.is_null() {
        lines.push(format!("args:\n{}", pretty_json_value(args)));
    }
    lines
}

pub fn format_tool_end_line(name: &str, result: &str) -> String {
    let display = if is_mcp_tool_name(name) {
        pretty_json_text(result)
    } else {
        result.to_string()
    };
    let lines: Vec<&str> = display.lines().collect();
    let take = if is_mcp_tool_name(name) { 24 } else { 3 };
    let preview: String = lines
        .iter()
        .take(take)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");
    let suffix = if lines.len() > take { "\n..." } else { "" };
    format!("[Tool: {name}] result:\n{preview}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::EventStream;
    use super::*;

    fn mock_stream(events: Vec<XyEvent>) -> EventStream {
        Box::pin(futures::stream::iter(events))
    }

    #[tokio::test]
    async fn text_delta_only_is_written_once() {
        let events = vec![
            XyEvent::TextDelta("Hello".into()),
            XyEvent::MessageUpdate { message: None },
            XyEvent::TextDelta("!".into()),
            XyEvent::MessageUpdate { message: None },
            XyEvent::TextDelta(" How".into()),
            XyEvent::MessageUpdate { message: None },
            XyEvent::AgentEnd {
                reason: None,
                messages: vec![],
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();

        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();

        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, "Hello! How\n");
    }

    #[tokio::test]
    async fn thinking_delta_does_not_pollute_stdout() {
        let events = vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta("I need to ".into()),
            XyEvent::ThinkingDelta("greet.".into()),
            XyEvent::TextDelta("Hi".into()),
            XyEvent::AgentEnd {
                reason: None,
                messages: vec![],
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();

        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();

        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, "Hi\n");
    }

    #[tokio::test]
    async fn thinking_delta_with_embedded_tags_is_not_double_wrapped() {
        let events = vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta("<think>".into()),
            XyEvent::ThinkingDelta("reasoning...".into()),
            XyEvent::ThinkingDelta("</think>".into()),
            XyEvent::TextDelta("Hi".into()),
            XyEvent::AgentEnd {
                reason: None,
                messages: vec![],
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();

        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();

        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, "Hi\n");
    }

    #[tokio::test]
    async fn thinking_block_writes_exactly_one_tag_pair_to_err() {
        let events = vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta("pondering".into()),
            XyEvent::TextDelta("ANSWER".into()),
            XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();
        let err_text = String::from_utf8(err_buf).unwrap();
        assert_eq!(err_text.matches("<think>").count(), 1, "{err_text}");
        assert_eq!(err_text.matches("</think>").count(), 1, "{err_text}");
        assert!(err_text.contains("pondering"), "{err_text}");
        assert_eq!(String::from_utf8(buf).unwrap(), "ANSWER\n");
    }

    #[tokio::test]
    async fn message_start_interrupting_an_open_block_closes_it_first() {
        let events = vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta("first".into()),
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta("second".into()),
            XyEvent::TextDelta("ANSWER".into()),
            XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();
        let err_text = String::from_utf8(err_buf).unwrap();
        assert_eq!(err_text.matches("<think>").count(), 2, "{err_text}");
        assert_eq!(err_text.matches("</think>").count(), 2, "{err_text}");
    }

    #[tokio::test]
    async fn late_closing_tag_fragment_does_not_double_close() {
        let events = vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta("reasoning".into()),
            XyEvent::ThinkingDelta("</think>".into()),
            XyEvent::TextDelta("ANSWER".into()),
            XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();
        let err_text = String::from_utf8(err_buf).unwrap();
        assert_eq!(err_text.matches("</think>").count(), 1, "{err_text}");
    }

    #[tokio::test]
    async fn stream_end_closes_an_unclosed_thinking_block() {
        let events = vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta("cut short".into()),
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();
        let err_text = String::from_utf8(err_buf).unwrap();
        assert_eq!(err_text.matches("<think>").count(), 1, "{err_text}");
        assert_eq!(err_text.matches("</think>").count(), 1, "{err_text}");
    }

    #[tokio::test]
    async fn error_event_returns_driver_err() {
        let events = vec![
            XyEvent::TextDelta("partial".into()),
            XyEvent::error_msg("provider blew up"),
            XyEvent::AgentEnd {
                reason: None,
                messages: vec![],
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();
        let err = render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .expect_err("Error event must fail the print stream");
        assert!(err.to_string().contains("provider blew up"), "got {err}");
        assert_eq!(String::from_utf8(buf).unwrap(), "partial");
    }

    #[tokio::test]
    async fn agent_end_without_error_returns_ok() {
        let events = vec![
            XyEvent::TextDelta("done".into()),
            XyEvent::AgentEnd {
                reason: None,
                messages: vec![],
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "done\n");
    }

    #[tokio::test]
    async fn multiple_turns_render_each_answer_and_keep_going() {
        let events = vec![
            XyEvent::AgentStart {
                session_id: "s".into(),
                model: "m".into(),
            },
            XyEvent::TextDelta("first".into()),
            XyEvent::AgentEnd {
                reason: None,
                messages: vec![],
            },
            XyEvent::AgentStart {
                session_id: "s".into(),
                model: "m".into(),
            },
            XyEvent::TextDelta("second".into()),
            XyEvent::AgentEnd {
                reason: None,
                messages: vec![],
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("first"), "{out}");
        assert!(out.contains("second"), "{out}");
    }

    #[tokio::test]
    async fn mcp_tool_prints_pretty_args_and_result_preview() {
        let events = vec![
            XyEvent::ToolExecutionStart {
                id: "m1".into(),
                name: "mcp__lspz__get_diagnostics".into(),
                args: serde_json::json!({"uri": "file:///x"}),
            },
            XyEvent::ToolExecutionEnd {
                id: "m1".into(),
                name: "mcp__lspz__get_diagnostics".into(),
                result: r#"{"content":[{"type":"text","text":"a"}],"isError":false}"#.into(),
                details: None,
                is_error: false,
            },
            XyEvent::AgentEnd {
                reason: None,
                messages: vec![],
            },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let mut err_buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf, &mut err_buf)
            .await
            .unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "\n");
    }

    #[test]
    fn mcp_format_helpers_pretty_print() {
        let start = format_tool_start_lines(
            "mcp__lspz__get_diagnostics",
            &serde_json::json!({"uri": "file:///x"}),
        );
        let joined = start.join("\n");
        assert!(
            joined.contains("[Tool: mcp__lspz__get_diagnostics]"),
            "{joined}"
        );
        assert!(joined.contains("args:\n"), "{joined}");
        assert!(joined.contains("\"uri\": \"file:///x\""), "{joined}");

        let end = format_tool_end_line(
            "mcp__lspz__get_diagnostics",
            r#"{"content":[{"type":"text","text":"a"}],"isError":false}"#,
        );
        assert!(end.contains("\"isError\": false"), "{end}");
        assert!(end.contains('\n'), "{end}");
    }
}
