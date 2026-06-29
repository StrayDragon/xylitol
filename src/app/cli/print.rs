//! Print mode — non-interactive, streaming stdout output.
//!
//! Subscribes to an XyEvent stream and renders text/tool output.

use std::io::{self, Write};

use crate::agent::facade::XyEvent;
use crate::app::core::driver::{Driver, EventStream};
use futures::StreamExt;

/// Run the agent in print mode with the given prompt.
pub(crate) async fn run_print(
    driver: &mut dyn Driver,
    prompt: &str,
    _session_id: &str, // reserved for future Driver.subscribe
) -> Result<(), String> {
    let mut stream = driver.run(prompt).await;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    render_stream(&mut stream, &mut handle).await
}

/// Render a stream of [`XyEvent`]s to a writer.
///
/// Extracted so print-mode event handling can be unit-tested without
/// capturing the real stdout.
async fn render_stream<W: Write>(stream: &mut EventStream, writer: &mut W) -> Result<(), String> {
    let mut in_thinking_block = false;
    let mut thinking_has_tags = false;

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
                if in_thinking_block {
                    if !thinking_has_tags {
                        let _ = write!(io::stderr(), "</think>");
                        let _ = io::stderr().flush();
                    }
                    in_thinking_block = false;
                    thinking_has_tags = false;
                }
            }
            XyEvent::TextDelta(text) => {
                if in_thinking_block {
                    if !thinking_has_tags {
                        let _ = write!(io::stderr(), "</think>");
                        let _ = io::stderr().flush();
                    }
                    in_thinking_block = false;
                    thinking_has_tags = false;
                }
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
            XyEvent::ToolExecutionStart { name, .. } => {
                eprintln!("\n[Tool: {name}]");
            }
            XyEvent::ToolExecutionUpdate { output, .. } => {
                eprint!("{output}");
                let _ = io::stderr().flush();
            }
            XyEvent::ToolExecutionEnd { name, result, .. } => {
                // Summarize result
                let preview: String = result.lines().take(3).collect::<Vec<_>>().join("\n");
                let suffix = if result.lines().count() > 3 {
                    "..."
                } else {
                    ""
                };
                eprintln!("[Tool: {name}] result:\n{preview}{suffix}");
            }
            XyEvent::Error(msg) => {
                eprintln!("\n[Error] {msg}");
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
            | XyEvent::SessionInfoChanged { .. } => {
                // Lifecycle metadata: silent in print mode.
            }
            XyEvent::AgentEnd { .. } => break,
        }
    }
    if in_thinking_block && !thinking_has_tags {
        let _ = write!(io::stderr(), "</think>");
        let _ = io::stderr().flush();
    }
    let _ = writeln!(writer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::driver::EventStream;

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
}
