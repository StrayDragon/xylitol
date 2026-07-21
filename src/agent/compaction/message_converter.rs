//! SessionEntry → AgentMessage conversion.
//!
//! The conversion lives on [`crate::domain::session_types::SessionEntry::as_agent_message`]
//! (c1210 unified seed path). This module keeps focused unit tests that previously
//! lived here.

#[cfg(test)]
mod tests {
    use crate::domain::message::{AgentMessage, AgentPart, EnvMessage, LlmMessage};
    use crate::domain::session_types::{
        BashExecutionEntry, EntryBase, MessageEntry, SessionEntry, fixture_message_json,
    };
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
            AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. }) => {
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

    #[test]
    fn lifts_legacy_top_level_bash() {
        let e = SessionEntry::BashExecution(BashExecutionEntry {
            base: EntryBase {
                entry_type: "bashExecution".into(),
                id: "b1".into(),
                parent_id: None,
                timestamp: "t".into(),
            },
            command: "ls".into(),
            output: "a".into(),
            exit_code: Some(0),
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: false,
        });
        let msg = e.as_agent_message().expect("lift");
        match msg {
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                command,
                output,
                exclude_from_context,
                ..
            }) => {
                assert_eq!(command, "ls");
                assert_eq!(output, "a");
                assert!(!exclude_from_context);
            }
            _ => panic!("expected Env bash"),
        }
    }

    #[test]
    fn skips_excluded_nested_bash_message() {
        let e = entry(json!({
            "role": "bashExecution",
            "command": "secret",
            "output": "x",
            "cancelled": false,
            "truncated": false,
            "exclude_from_context": true,
        }));
        assert!(e.as_agent_message().is_none());
    }

    #[test]
    fn compaction_projects_to_env() {
        use crate::domain::session_types::CompactionEntry;
        let e = SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: None,
                timestamp: "t".into(),
            },
            summary: "sum".into(),
            first_kept_entry_id: "m1".into(),
            tokens_before: 100,
            details: None,
            from_hook: None,
        });
        let msg = e.as_agent_message().expect("compaction");
        assert_eq!(msg.role_name(), "compactionSummary");
        assert!(msg.text().contains("sum"));
    }
}
