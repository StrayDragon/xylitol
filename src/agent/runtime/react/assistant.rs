//! Streaming assistant message assembly helpers for the ReAct loop.

use std::sync::Mutex;

use serde_json::Value;

use crate::protocol::lifecycle::XyEvent;
use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

pub(crate) fn upsert_streaming_tool(
    tools: &mut Vec<(String, String, Value)>,
    id: String,
    name: String,
    args: Value,
) {
    if let Some(slot) = tools
        .iter_mut()
        .find(|(existing_id, _, _)| existing_id == &id)
    {
        slot.1 = name;
        slot.2 = args;
    } else {
        tools.push((id, name, args));
    }
}

pub(crate) fn streaming_assistant_parts(
    text: &str,
    thinking: &str,
    thinking_signature: Option<&str>,
    tool_calls: &[(String, String, Value)],
) -> Vec<AgentPart> {
    let mut parts = Vec::new();
    if !thinking.is_empty() || thinking_signature.is_some() {
        parts.push(AgentPart::Thinking {
            thinking: thinking.to_string(),
            redacted: false,
            thinking_signature: thinking_signature.map(str::to_string),
        });
    }
    if !text.is_empty() {
        parts.push(AgentPart::text(text.to_string()));
    }
    for (id, name, args) in tool_calls {
        parts.push(AgentPart::ToolCall {
            id: id.clone(),
            name: name.clone(),
            arguments: args.clone(),
        });
    }
    parts
}

pub(crate) fn partial_assistant_message(
    text: &str,
    thinking: &str,
    thinking_signature: Option<&str>,
    tool_calls: &[(String, String, Value)],
) -> AgentMessage {
    build_assistant_message(
        streaming_assistant_parts(text, thinking, thinking_signature, tool_calls),
        None,
        None,
        String::new(),
        String::new(),
        None,
    )
}

pub(crate) fn build_assistant_message(
    content: Vec<AgentPart>,
    stop_reason: Option<crate::protocol::message::XyStopReason>,
    usage: Option<crate::protocol::message::XyUsage>,
    provider: String,
    model: String,
    error_message: Option<String>,
) -> AgentMessage {
    AgentMessage::Llm(LlmMessage::AssistantMessage {
        content,
        stop_reason,
        usage,
        api: String::new(),
        provider,
        model,
        response_id: None,
        error_message,
        timestamp: crate::protocol::message::now_ms(),
        diagnostics: Vec::new(),
    })
}

pub(crate) fn streaming_message_update(
    text: &str,
    thinking: &str,
    thinking_signature: Option<&str>,
    tool_calls: &[(String, String, Value)],
) -> XyEvent {
    XyEvent::MessageUpdate {
        text: text.to_string(),
        thinking: if thinking.is_empty() {
            None
        } else {
            Some(thinking.to_string())
        },
        message: Some(partial_assistant_message(
            text,
            thinking,
            thinking_signature,
            tool_calls,
        )),
    }
}

pub(crate) fn current_provider_model(
    model_manager: &Mutex<crate::agent::model::manager::ModelManager>,
) -> (String, String) {
    let mm = crate::utils::lock_mutex(model_manager);
    mm.current_model()
        .map(|m| (m.config.provider_name().to_string(), m.config.model.clone()))
        .unwrap_or_default()
}
