use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoStaticStr;

use crate::error::XyError;
use crate::message::AgentMessage;
use crate::model::ContextTokenEstimate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XyEventError {
    pub kind: String,
    pub message: String,
}

impl XyEventError {
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }

    pub fn aborted() -> Self {
        Self::new("Aborted", "aborted")
    }

    pub fn from_xy(err: &XyError) -> Self {
        Self::new(err.kind(), err.to_string())
    }

    pub fn is_aborted(&self) -> bool {
        self.kind == "Aborted" || self.message == "aborted"
    }
}

impl From<&XyError> for XyEventError {
    fn from(err: &XyError) -> Self {
        Self::from_xy(err)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoStaticStr)]
#[serde(tag = "type", rename_all = "camelCase")]
#[strum(serialize_all = "snake_case")]
pub enum XyEvent {
    AgentStart {
        session_id: String,
        model: String,
    },
    AgentEnd {
        messages: Vec<AgentMessage>,
    },

    TurnStart {
        turn_index: u32,
    },
    TurnEnd {
        turn_index: u32,
    },

    MessageStart {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<AgentMessage>,
    },
    MessageUpdate {
        text: String,
        thinking: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<AgentMessage>,
    },
    MessageEnd {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<AgentMessage>,
    },
    TextDelta(String),
    ThinkingDelta(String),

    ToolExecutionStart {
        id: String,
        name: String,
        args: Value,
    },
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
    ToolExecutionEnd {
        id: String,
        name: String,
        result: String,
        #[serde(default)]
        is_error: bool,
    },

    CompactionStart {
        reason: String,
    },
    CompactionEnd {
        result: Option<String>,
        #[serde(default)]
        aborted: bool,
        #[serde(default)]
        reason: String,
        #[serde(default)]
        will_retry: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens_before: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens_after: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
    },

    ContextTokenSettlement {
        estimate: ContextTokenEstimate,
        reason: String,
        generation: u64,
    },

    ModelSelect {
        provider: String,
        model_id: String,
    },
    ThinkingLevelChanged {
        level: String,
    },

    QueueUpdate {
        steer_count: usize,
        follow_up_count: usize,
    },

    TodoUpdated {
        list: crate::session::TodoList,
    },

    AutoRetryStart {
        attempt: u32,
        max_retries: u32,
        delay_ms: u64,
    },
    AutoRetryEnd {
        success: bool,
        attempt: u32,
    },

    Error(XyEventError),

    SessionInfoChanged {
        key: String,
        value: Value,
    },
}

impl XyEvent {
    pub fn aborted() -> Self {
        Self::Error(XyEventError::aborted())
    }

    pub fn error_msg(message: impl Into<String>) -> Self {
        Self::Error(XyEventError::new("Message", message))
    }

    pub fn description(&self) -> &'static str {
        self.into()
    }
}
