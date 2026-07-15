//! Session → LLM projection: fold environment roles into [`LlmMessage`] rows.
//!
//! [`AgentMessage`] remains the session SSOT (`Llm` ∪ `Env`). Before any
//! provider call, history MUST pass through [`project_for_llm`].

use serde_json::Value;

use super::message::{AgentMessage, EnvMessage, LlmMessage, now_ms};

/// Project session history into LLM-visible [`LlmMessage`] rows.
pub fn project_for_llm(messages: &[AgentMessage]) -> Vec<LlmMessage> {
    let mut out = Vec::with_capacity(messages.len());
    for msg in messages {
        match msg {
            AgentMessage::Llm(m) => out.push(m.clone()),
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                command,
                output,
                exclude_from_context,
                ..
            }) => {
                if *exclude_from_context {
                    continue;
                }
                out.push(user_text(format!("$ {command}\n{output}")));
            }
            AgentMessage::Env(
                EnvMessage::CompactionSummaryMessage { summary, .. }
                | EnvMessage::BranchSummaryMessage { summary, .. },
            ) => {
                out.push(user_text(format!("[Context summary: {summary}]")));
            }
            AgentMessage::Env(EnvMessage::CustomMessage { content, .. }) => {
                if let Some(text) = custom_text(content)
                    && !text.is_empty()
                {
                    out.push(user_text(text));
                }
            }
        }
    }
    out
}

fn user_text(text: String) -> LlmMessage {
    LlmMessage::UserMessage {
        content: vec![super::message::AgentPart::text(text)],
        timestamp: now_ms(),
    }
}

fn custom_text(content: &Value) -> Option<String> {
    match content {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::{AgentMessage, AgentPart, EnvMessage};

    #[test]
    fn bash_folds_to_user_llm() {
        let history = vec![
            AgentMessage::user("hi"),
            AgentMessage::bash("ls", "a.txt", Some(0)),
        ];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 2);
        assert_eq!(projected[1].role_name(), "user");
        assert!(projected[1].text().contains("ls"));
    }

    #[test]
    fn bash_excluded_from_context_is_skipped() {
        let history = vec![AgentMessage::Env(EnvMessage::BashExecutionMessage {
            command: "secret".into(),
            output: "x".into(),
            exit_code: None,
            cancelled: false,
            truncated: false,
            exclude_from_context: true,
        })];
        assert!(project_for_llm(&history).is_empty());
    }

    #[test]
    fn compaction_and_branch_fold() {
        let history = vec![
            AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                summary: "compressed".into(),
                tokens_before: 100,
                tokens_after: 10,
                read_files: None,
                modified_files: None,
            }),
            AgentMessage::Env(EnvMessage::BranchSummaryMessage {
                summary: "forked".into(),
                from_id: "abc".into(),
            }),
        ];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 2);
        assert!(projected[0].text().contains("compressed"));
        assert!(projected[1].text().contains("forked"));
    }

    #[test]
    fn tool_result_passthrough_as_llm() {
        let history = vec![AgentMessage::tool_result(
            "c1",
            "read",
            vec![AgentPart::text("ok")],
            false,
        )];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].role_name(), "toolResult");
    }
}
