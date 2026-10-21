use crate::message::{AgentMessage, AgentPart, EnvLlmProjection, XyLlmMessage, now_ms};

pub fn fold_bash_for_llm(command: &str, output: &str) -> String {
    format!("$ {command}\n{output}")
}

pub fn fold_context_summary_for_llm(summary: &str) -> String {
    format!("[Context summary: {summary}]")
}

pub fn project_for_llm(messages: &[AgentMessage]) -> Vec<XyLlmMessage> {
    let mut out = Vec::with_capacity(messages.len());
    for msg in messages {
        match msg {
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
