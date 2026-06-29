//! Event vocabulary — core → client messages.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::lifecycle::XyEvent;

/// An event from the core: either a response to a command or a streamed
/// occurrence during a turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
    },
    Response {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
    TextDelta {
        text: String,
    },
    ToolStart {
        id: String,
        name: String,
    },
    ToolEnd {
        id: String,
        name: String,
        result: String,
    },
    AgentEnd,
    ModelSelect {
        provider: String,
        model_id: String,
    },
    CompactionStart {
        reason: String,
    },
    /// Acknowledgment of a Subscribe command.
    Subscribed {
        session_id: String,
        seq: u64,
    },
    XyBashResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        output: String,
        exit_code: Option<i32>,
        cancelled: bool,
        truncated: bool,
    },
    /// Turn started.
    TurnStart {
        turn_index: u32,
    },
    /// Turn ended.
    TurnEnd {
        turn_index: u32,
    },
    /// Message started.
    MessageStart {
        role: String,
    },
    /// Message ended.
    MessageEnd {
        role: String,
    },
    /// Streaming message update (replaces previous text/thinking for this message).
    MessageUpdate {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
    },
    /// Streaming tool execution output.
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
    /// Compaction completed.
    CompactionEnd,
}

impl From<&XyEvent> for Event {
    fn from(event: &XyEvent) -> Self {
        match event {
            XyEvent::TextDelta(text) => Event::TextDelta { text: text.clone() },
            XyEvent::TurnStart { turn_index } => Event::TurnStart {
                turn_index: *turn_index,
            },
            XyEvent::TurnEnd { turn_index } => Event::TurnEnd {
                turn_index: *turn_index,
            },
            XyEvent::MessageStart { role, .. } => Event::MessageStart { role: role.clone() },
            XyEvent::MessageEnd { role, .. } => Event::MessageEnd { role: role.clone() },
            XyEvent::MessageUpdate { text, thinking, .. } => Event::MessageUpdate {
                text: text.clone(),
                thinking: thinking.clone(),
            },
            XyEvent::ThinkingDelta(_) => Event::MessageUpdate {
                text: String::new(),
                thinking: None,
            },
            XyEvent::ToolExecutionStart { id, name, .. } => Event::ToolStart {
                id: id.clone(),
                name: name.clone(),
            },
            XyEvent::ToolExecutionEnd {
                id, name, result, ..
            } => Event::ToolEnd {
                id: id.clone(),
                name: name.clone(),
                result: result.clone(),
            },
            XyEvent::ToolExecutionUpdate { id, output } => Event::ToolExecutionUpdate {
                id: id.clone(),
                output: output.clone(),
            },
            XyEvent::ModelSelect { provider, model_id } => Event::ModelSelect {
                provider: provider.clone(),
                model_id: model_id.clone(),
            },
            XyEvent::CompactionStart { reason } => Event::CompactionStart {
                reason: reason.clone(),
            },
            XyEvent::CompactionEnd { .. } => Event::CompactionEnd,
            XyEvent::AgentEnd { .. } => Event::AgentEnd,
            XyEvent::Error(msg) => Event::Error {
                id: None,
                message: msg.clone(),
            },
            _ => Event::Response {
                id: None,
                payload: None,
            },
        }
    }
}

impl TryFrom<&Event> for XyEvent {
    type Error = String;

    fn try_from(event: &Event) -> Result<Self, <Self as TryFrom<&Event>>::Error> {
        match event {
            Event::TextDelta { text } => Ok(XyEvent::TextDelta(text.clone())),
            Event::TurnStart { turn_index } => Ok(XyEvent::TurnStart {
                turn_index: *turn_index,
            }),
            Event::TurnEnd { turn_index } => Ok(XyEvent::TurnEnd {
                turn_index: *turn_index,
            }),
            Event::MessageStart { role } => Ok(XyEvent::MessageStart {
                role: role.clone(),
                message: None,
            }),
            Event::MessageEnd { role } => Ok(XyEvent::MessageEnd {
                role: role.clone(),
                message: None,
            }),
            Event::MessageUpdate { text, thinking } => Ok(XyEvent::MessageUpdate {
                text: text.clone(),
                thinking: thinking.clone(),
                message: None,
            }),
            Event::ToolStart { id, name } => Ok(XyEvent::ToolExecutionStart {
                id: id.clone(),
                name: name.clone(),
                args: Value::Null,
            }),
            Event::ToolEnd { id, name, result } => Ok(XyEvent::ToolExecutionEnd {
                id: id.clone(),
                name: name.clone(),
                result: result.clone(),
                is_error: false,
            }),
            Event::ToolExecutionUpdate { id, output } => Ok(XyEvent::ToolExecutionUpdate {
                id: id.clone(),
                output: output.clone(),
            }),
            Event::ModelSelect { provider, model_id } => Ok(XyEvent::ModelSelect {
                provider: provider.clone(),
                model_id: model_id.clone(),
            }),
            Event::CompactionStart { reason } => Ok(XyEvent::CompactionStart {
                reason: reason.clone(),
            }),
            Event::CompactionEnd => Ok(XyEvent::CompactionEnd {
                result: None,
                aborted: false,
            }),
            Event::AgentEnd => Ok(XyEvent::AgentEnd {
                messages: Vec::new(),
            }),
            Event::Error { message, .. } => Ok(XyEvent::Error(message.clone())),
            other => Err(format!(
                "event variant not convertible to XyEvent: {other:?}"
            )),
        }
    }
}
