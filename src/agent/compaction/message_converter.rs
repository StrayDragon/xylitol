//! SessionEntry → AgentMessage conversion.
//!
//! Provides [`SessionEntry::as_agent_message()`] for converting persisted
//! session entries back into runtime agent messages.

use crate::domain::message::AgentMessage;
use crate::domain::session_types::SessionEntry;

impl SessionEntry {
    /// Convert a SessionEntry to `AgentMessage` if it contains conversation content.
    ///
    /// c646: only tagged AgentMessage JSON is accepted (E1 — no legacy untagged content).
    pub fn as_agent_message(&self) -> Option<AgentMessage> {
        match self {
            SessionEntry::Message(msg) => {
                match serde_json::from_value::<AgentMessage>(msg.message.clone()) {
                    Ok(agent_msg) => Some(agent_msg),
                    Err(e) => {
                        log::warn!(target: "xylitol::session", "skip message entry: AgentMessage deserialize failed (c646 tagged wire only) error={}", e);
                        None
                    }
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::message::AgentPart;
    use crate::domain::session_types::{EntryBase, MessageEntry, fixture_message_json};
    use serde_json::{Value, json};

    fn entry(message: Value) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "e1".into(),
                parent_id: None,
                timestamp: "t".into(),
            },
            message,
        })
    }

    #[test]
    fn converts_agent_message_content_shape() {
        let e = entry(fixture_message_json("user", "hello"));
        let msg = e.as_agent_message().expect("user");
        assert_eq!(msg.role_name(), "user");
        assert_eq!(msg.text(), "hello");
    }

    #[test]
    fn converts_assistant_thinking_and_text() {
        let e = entry(json!({
            "role": "assistant",
            "content": [
                { "type": "thinking", "thinking": "reason" },
                { "type": "text", "text": "answer" }
            ],
            "timestamp": 0u64,
            "api": "",
            "provider": "",
            "model": "",
        }));
        let msg = e.as_agent_message().expect("assistant");
        match msg {
            crate::domain::message::AgentMessage::Llm(
                crate::domain::message::LlmMessage::AssistantMessage { content, .. },
            ) => {
                assert!(matches!(
                    content.as_slice(),
                    [
                        AgentPart::Thinking { thinking, .. },
                        AgentPart::Text { text }
                    ] if thinking == "reason" && text == "answer"
                ));
            }
            _ => panic!("expected assistant"),
        }
    }

    #[test]
    fn rejects_untagged_thinking_content() {
        let e = entry(json!({
            "role": "assistant",
            "content": [
                { "redacted": false, "text": "old thinking" },
                "answer"
            ],
            "timestamp": 0u64,
            "api": "",
            "provider": "",
            "model": "",
        }));
        assert!(e.as_agent_message().is_none());
    }
}
