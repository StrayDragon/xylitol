//! Print mode — non-interactive, streaming stdout output.
//!
//! Subscribes to an XyEvent stream and renders text/tool output.

use std::io::{self, Write};

use crate::tool_display::{is_mcp_tool_name, pretty_json_text, pretty_json_value};
use futures::StreamExt;
use std::pin::Pin;
use xylitol_runtime::XyEvent;

/// Stream of lifecycle events from the in-process driver.
pub type EventStream = Pin<Box<dyn futures::Stream<Item = XyEvent> + Send>>;

/// Print-mode failure (maps from the composition-root driver at the CLI).
#[derive(Debug, thiserror::Error)]
pub enum PrintError {
    #[error("{0}")]
    Message(String),
}

/// Run the agent in print mode with the given prompt.
pub async fn run_print<S>(stream: &mut S, _session_id: &str) -> Result<(), PrintError>
where
    S: futures::Stream<Item = XyEvent> + Unpin,
{
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    render_stream(stream, &mut handle).await
}

/// Render a stream of [`XyEvent`]s to a writer.
///
/// Extracted so print-mode event handling can be unit-tested without
/// capturing the real stdout.
pub async fn render_stream<S, W: Write>(stream: &mut S, writer: &mut W) -> Result<(), PrintError>
where
    S: futures::Stream<Item = XyEvent> + Unpin,
{
    let mut in_thinking_block = false;
    let mut thinking_has_tags = false;

    // Close an open thinking block: emit the closing `</think>` when the model
    // did not tag the block itself, then clear block state. Shared by the
    // MessageEnd / TextDelta / Error / stream-end paths below.
    let close_thinking_block = |in_thinking_block: &mut bool, thinking_has_tags: &mut bool| {
        if *in_thinking_block {
            if !*thinking_has_tags {
                let _ = write!(io::stderr(), "</think>");
                let _ = io::stderr().flush();
            }
            *in_thinking_block = false;
            *thinking_has_tags = false;
        }
    };

    while let Some(event) = stream.next().await {
        match event {
            XyEvent::TurnStart { turn_index } => {
                eprintln!("\n[Turn {turn_index}]");
            }
            XyEvent::TurnEnd { turn_index } => {
                eprintln!("\n[Turn {turn_index} end]");
            }
            XyEvent::MessageStart { .. } => {
                // Reset thinking state for a new assistant message.
                in_thinking_block = false;
                thinking_has_tags = false;
            }
            XyEvent::MessageEnd { .. } => {
                close_thinking_block(&mut in_thinking_block, &mut thinking_has_tags);
            }
            XyEvent::TextDelta(text) => {
                close_thinking_block(&mut in_thinking_block, &mut thinking_has_tags);
                let _ = write!(writer, "{text}");
                let _ = writer.flush();
            }
            XyEvent::ThinkingDelta(text) => {
                if !in_thinking_block {
                    // Some models (e.g. Qwen with chat-template thinking) emit
                    // their own <think>...</think> tags inside the reasoning
                    // stream. If we see a <think> tag, do not wrap the block
                    // again; otherwise add our own tags so stderr viewers can
                    // distinguish reasoning from the final answer.
                    thinking_has_tags = text.contains("<think>");
                    if !thinking_has_tags {
                        let _ = write!(io::stderr(), "<think>");
                    }
                    in_thinking_block = true;
                }
                let _ = write!(io::stderr(), "{text}");
                let _ = io::stderr().flush();
            }
            XyEvent::MessageUpdate { .. } => {
                // MessageUpdate carries the *accumulated* full message state
                // (not a delta). In print mode we stream only TextDelta
                // increments to stdout; writing this variant would repeat
                // every prefix and produce garbled output like
                // "HelloHello!Hello! How...".
            }
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
            XyEvent::Error(err) => {
                let msg = &err.message;
                eprintln!("\n[Error] {msg}");
                close_thinking_block(&mut in_thinking_block, &mut thinking_has_tags);
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
            // atd9: print surface has no checklist UI; todo state stays in
            // tool results + SSOT.
            | XyEvent::TodoUpdated { .. } => {
                // Lifecycle metadata: silent in print mode.
            }
            XyEvent::AgentEnd { .. } => break,
        }
    }
    close_thinking_block(&mut in_thinking_block, &mut thinking_has_tags);
    let _ = writeln!(writer);
    Ok(())
}

pub fn format_tool_start_lines(name: &str, args: &serde_json::Value) -> Vec<String> {
    let mut lines = vec![format!("\n[Tool: {name}]")];
    // c1460: MCP call args on stderr for debug (pretty JSON).
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
    // Summarize: MCP pretty may be long — keep a short preview + ellipsis.
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
            XyEvent::MessageUpdate {
                text: "Hello".into(),
                thinking: None,
                message: None,
            },
            XyEvent::TextDelta("!".into()),
            XyEvent::MessageUpdate {
                text: "Hello!".into(),
                thinking: None,
                message: None,
            },
            XyEvent::TextDelta(" How".into()),
            XyEvent::MessageUpdate {
                text: "Hello! How".into(),
                thinking: None,
                message: None,
            },
            XyEvent::AgentEnd { messages: vec![] },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();

        render_stream(&mut stream, &mut buf).await.unwrap();

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
            XyEvent::AgentEnd { messages: vec![] },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();

        render_stream(&mut stream, &mut buf).await.unwrap();

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
            XyEvent::AgentEnd { messages: vec![] },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();

        render_stream(&mut stream, &mut buf).await.unwrap();

        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, "Hi\n");
    }

    #[tokio::test]
    async fn error_event_returns_driver_err() {
        let events = vec![
            XyEvent::TextDelta("partial".into()),
            XyEvent::error_msg("provider blew up"),
            XyEvent::AgentEnd { messages: vec![] },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        let err = render_stream(&mut stream, &mut buf)
            .await
            .expect_err("Error event must fail the print stream");
        assert!(err.to_string().contains("provider blew up"), "got {err}");
        assert_eq!(String::from_utf8(buf).unwrap(), "partial");
    }

    #[tokio::test]
    async fn agent_end_without_error_returns_ok() {
        let events = vec![
            XyEvent::TextDelta("done".into()),
            XyEvent::AgentEnd { messages: vec![] },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf).await.unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "done\n");
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
                is_error: false,
            },
            XyEvent::AgentEnd { messages: vec![] },
        ];
        let mut stream = mock_stream(events);
        let mut buf: Vec<u8> = Vec::new();
        render_stream(&mut stream, &mut buf).await.unwrap();
        // MCP fixed-zone output goes to stderr; stdout only gets final newline from AgentEnd path.
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
