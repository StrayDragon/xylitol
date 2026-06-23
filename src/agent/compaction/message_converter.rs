//! SessionEntry → AgentMessage conversion.
//!
//! Provides [`SessionEntry::as_agent_message()`] for converting persisted
//! session entries back into runtime agent messages.

use crate::core::message::{AgentMessage, AgentPart, now_ms};
use crate::infra::session::types::SessionEntry;

impl SessionEntry {
    /// Convert a SessionEntry to `AgentMessage` if it contains conversation content.
    pub fn as_agent_message(&self) -> Option<AgentMessage> {
        match self {
            SessionEntry::Message(msg) => {
                if let Ok(agent_msg) =
                    serde_json::from_value::<AgentMessage>(msg.message.clone())
                {
                    return Some(agent_msg);
                }
                let role = msg.message.get("role")?.as_str()?;
                let parts: Vec<AgentPart> = msg
                    .message
                    .get("parts")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| {
                                let typ = p.get("type")?.as_str()?;
                                match typ {
                                    "Text" => Some(AgentPart::Text(
                                        p.get("text")?.as_str()?.to_string(),
                                    )),
                                    "Thinking" => Some(AgentPart::Thinking {
                                        text: p.get("thinking")?.as_str()?.to_string(),
                                        redacted: false,
                                        signature: None,
                                    }),
                                    "FunctionCall" => Some(AgentPart::ToolCall {
                                        name: p.get("name")?.as_str()?.to_string(),
                                        arguments: p.get("args")?.clone(),
                                        id: p.get("id")?.as_str()?.to_string(),
                                    }),
                                    "FunctionResponse" => Some(AgentPart::ToolResult {
                                        tool_use_id: p.get("id")?.as_str()?.to_string(),
                                        content: vec![AgentPart::Text(
                                            p.get("result")?.as_str()?.to_string(),
                                        )],
                                        is_error: false,
                                    }),
                                    _ => None,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                match role {
                    "user" => Some(AgentMessage::UserMessage {
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
                    "system" => Some(AgentMessage::UserMessage {
                        content: parts,
                        timestamp: now_ms(),
                    }),
                    "tool" => Some(AgentMessage::ToolResultMessage {
                        tool_use_id: String::new(),
                        tool_name: String::new(),
                        content: parts,
                        details: None,
                        is_error: false,
                        timestamp: now_ms(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
