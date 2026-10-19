//! Session → LLM projection: fold environment roles into [`XyLlmMessage`] rows.
//!
//! [`AgentMessage`] remains the session SSOT (`Llm` ∪ `Env`). Before any
//! provider call, history MUST pass through [`project_for_llm`].

use crate::message::{AgentMessage, AgentPart, EnvLlmProjection, XyLlmMessage, now_ms};

/// Stable bash → LLM user-row fold (`$ {command}\n{output}`).
pub fn fold_bash_for_llm(command: &str, output: &str) -> String {
    format!("$ {command}\n{output}")
}

/// Stable compaction / branch summary → LLM user-row fold.
pub fn fold_context_summary_for_llm(summary: &str) -> String {
    format!("[Context summary: {summary}]")
}

/// Project session history into LLM-visible [`XyLlmMessage`] rows.
pub fn project_for_llm(messages: &[AgentMessage]) -> Vec<XyLlmMessage> {
    let mut out = Vec::with_capacity(messages.len());
    for msg in messages {
        match msg {
            // c1595 / pi: aborted|error assistants stay in session scrollback but
            // MUST NOT be replayed to the next LLM call (`XyLlmMessage::is_error`).
            AgentMessage::Llm(m) => {
                if m.is_error() {
                    continue;
                }
                out.push(m.clone());
            }
            AgentMessage::Env(env) => match env.llm_projection() {
                Some(EnvLlmProjection::Bash { command, output }) => {
                    out.push(user_text(fold_bash_for_llm(&command, &output)));
                }
                Some(EnvLlmProjection::ContextSummary { summary }) => {
                    out.push(user_text(fold_context_summary_for_llm(&summary)));
                }
                Some(EnvLlmProjection::CustomText { text }) => out.push(user_text(text)),
                None => {}
            },
        }
    }
    out
}

pub fn user_text(text: String) -> XyLlmMessage {
    XyLlmMessage::UserMessage {
        content: vec![AgentPart::text(text)],
        timestamp: now_ms(),
    }
}
