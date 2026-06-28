//! Print mode — non-interactive, streaming stdout output.
//!
//! Subscribes to an XyEvent stream and renders text/tool output.

use std::io::{self, Write};

use crate::agent::facade::XyEvent;
use crate::app::driver::{Driver, EventStream};
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
    while let Some(event) = stream.next().await {
        match event {
            XyEvent::TurnStart { turn_index } => {
                eprintln!("\n[Turn {turn_index}]");
            }
            XyEvent::TurnEnd { turn_index } => {
                eprintln!("\n[Turn {turn_index} end]");
            }
            XyEvent::MessageStart { .. } => {
                // Silent: don't interrupt the output stream.
            }
            XyEvent::MessageEnd { .. } => {
                // Silent: don't interrupt the output stream.
            }
            XyEvent::TextDelta(text) => {
                let _ = write!(writer, "{text}");
                let _ = writer.flush();
            }
            XyEvent::ThinkingDelta(_) => {}
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
    let _ = writeln!(writer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::driver::EventStream;

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
}
