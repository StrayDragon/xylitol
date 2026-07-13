//! SessionEntry → AgentMessage conversion.
//!
//! Provides [`SessionEntry::as_agent_message()`] for converting persisted
//! session entries back into runtime agent messages.

use crate::domain::message::{AgentMessage, AgentPart, now_ms};
use crate::domain::session_types::SessionEntry;
use serde_json::Value;

impl SessionEntry {
    /// Convert a SessionEntry to `AgentMessage` if it contains conversation content.
    pub fn as_agent_message(&self) -> Option<AgentMessage> {
        match self {
            SessionEntry::Message(msg) => {
                if let Ok(agent_msg) = serde_json::from_value::<AgentMessage>(msg.message.clone()) {
                    return Some(agent_msg);
                }
                let role = msg.message.get("role")?.as_str()?;
                let parts = parse_message_parts(&msg.message);
                match role {
                    "user" | "system" => Some(AgentMessage::UserMessage {
                        content: parts,
                        timestamp: now_ms(),
                    }),
                    "assistant" => Some(AgentMessage::AssistantMessage {
                        content: parts,
                        stop_reason: None,
                        usage: None,
                        api: String::new(),
                        provider: String::new(),
                        model: String::new(),
                        response_id: None,
                        error_message: None,
                        timestamp: now_ms(),
                        diagnostics: Vec::new(),
                    }),
                    "tool" | "toolResult" => Some(AgentMessage::ToolResultMessage {
                        tool_use_id: msg
                            .message
                            .get("tool_use_id")
                            .or_else(|| msg.message.get("toolUseId"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        tool_name: msg
                            .message
                            .get("tool_name")
                            .or_else(|| msg.message.get("toolName"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        content: parts,
                        details: None,
                        is_error: msg
                            .message
                            .get("is_error")
                            .or_else(|| msg.message.get("isError"))
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        timestamp: now_ms(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn parse_message_parts(message: &Value) -> Vec<AgentPart> {
    let Some(arr) = message
        .get("content")
        .or_else(|| message.get("parts"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    arr.iter().filter_map(parse_one_part).collect()
}

fn parse_one_part(p: &Value) -> Option<AgentPart> {
    if let Ok(part) = serde_json::from_value::<AgentPart>(p.clone()) {
        return Some(part);
    }
    // Legacy typed parts (pre-AgentMessage serde).
    let typ = p.get("type")?.as_str()?;
    match typ {
        "text" | "Text" => Some(AgentPart::Text(p.get("text")?.as_str()?.to_string())),
        "thinking" | "Thinking" => Some(AgentPart::Thinking {
            text: p
                .get("text")
                .or_else(|| p.get("thinking"))
                .and_then(Value::as_str)?
                .to_string(),
            redacted: p.get("redacted").and_then(Value::as_bool).unwrap_or(false),
            signature: p
                .get("signature")
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
        "toolCall" | "FunctionCall" => Some(AgentPart::ToolCall {
            id: p
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            name: p.get("name")?.as_str()?.to_string(),
            arguments: p
                .get("arguments")
                .or_else(|| p.get("args"))
                .cloned()
                .unwrap_or(Value::Null),
        }),
        "toolResult" | "FunctionResponse" => Some(AgentPart::ToolResult {
            tool_use_id: p
                .get("tool_use_id")
                .or_else(|| p.get("toolUseId"))
                .or_else(|| p.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            content: vec![AgentPart::Text(
                p.get("result")
                    .or_else(|| p.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            )],
            is_error: p
                .get("is_error")
                .or_else(|| p.get("isError"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session_types::{EntryBase, MessageEntry, fixture_message_json};
    use serde_json::json;

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
        let msg = e.as_agent_message().expect("convert");
        assert_eq!(msg.role_name(), "user");
        assert_eq!(msg.text(), "hello");
    }

    #[test]
    fn converts_assistant_with_tool_call() {
        let e = entry(json!({
            "role": "assistant",
            "content": [
                "看一下",
                { "id": "1", "name": "read", "arguments": { "path": "a.rs" } }
            ],
            "timestamp": 0u64,
        }));
        let msg = e.as_agent_message().expect("convert");
        assert_eq!(msg.role_name(), "assistant");
        assert_eq!(msg.text(), "看一下");
        assert!(msg.content().iter().any(AgentPart::is_tool_call));
    }

    #[test]
    fn converts_legacy_parts_function_call() {
        let e = entry(json!({
            "role": "assistant",
            "parts": [{
                "type": "FunctionCall",
                "id": "c1",
                "name": "bash",
                "args": { "command": "ls" }
            }]
        }));
        let msg = e.as_agent_message().expect("convert");
        let AgentPart::ToolCall {
            name, arguments, ..
        } = &msg.content()[0]
        else {
            panic!("expected tool call");
        };
        assert_eq!(name, "bash");
        assert_eq!(arguments["command"], "ls");
    }
}
