//! Print mode — non-interactive, streaming stdout output.
//!
//! Subscribes to an AgentEvent stream and renders text/tool output.

use std::io::{self, Write};

use crate::agent::facade::AgentEvent;
use crate::interactive::driver::{Driver, EventStream};
use futures::StreamExt;

/// Run the agent in print mode with the given prompt.
pub(crate) async fn run_print(
    driver: &mut dyn Driver,
    prompt: &str,
    session_id: &str,
) -> Result<(), String> {
    let mut stream = driver.run(prompt).await;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    while let Some(event) = stream.next().await {
        match event {
            AgentEvent::TextDelta(text) => {
                let _ = write!(handle, "{text}");
                let _ = handle.flush();
            }
            AgentEvent::ThinkingDelta(_) => {}
            AgentEvent::ToolExecutionStart { name, .. } => {
                eprintln!("\n[Tool: {name}]");
            }
            AgentEvent::ToolExecutionEnd { name, result, .. } => {
                // Summarize result
                let preview: String = result.lines().take(3).collect::<Vec<_>>().join("\n");
                let suffix = if result.lines().count() > 3 {
                    "..."
                } else {
                    ""
                };
                eprintln!("[Tool: {name}] result:\n{preview}{suffix}");
            }
            AgentEvent::Error(msg) => {
                eprintln!("\n[Error] {msg}");
            }
            AgentEvent::CompactionStart { reason } => {
                eprintln!("\n[Compaction] {reason}");
            }
            AgentEvent::ModelSelect { model_id, .. } => {
                eprintln!("\n[Model] switched to {model_id}");
            }
            AgentEvent::AgentEnd { .. } => break,
            _ => {}
        }
    }
    let _ = writeln!(handle);
    Ok(())
}
