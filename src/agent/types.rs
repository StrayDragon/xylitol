use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XyContent {
    pub role: XyRole,
    pub parts: Vec<XyPart>,
}

impl XyContent {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: XyRole::User,
            parts: vec![XyPart::Text(text.into())],
        }
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: XyRole::System,
            parts: vec![XyPart::Text(text.into())],
        }
    }

    pub fn assistant(parts: Vec<XyPart>) -> Self {
        Self {
            role: XyRole::Assistant,
            parts,
        }
    }

    pub fn tool_result(name: String, result: String, id: String) -> Self {
        Self {
            role: XyRole::Tool,
            parts: vec![XyPart::FunctionResponse { name, result, id }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum XyRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XyPart {
    Text(String),
    Thinking(String),
    FunctionCall {
        name: String,
        args: Value,
        id: String,
    },
    FunctionResponse {
        name: String,
        result: String,
        id: String,
    },
}

#[derive(Debug, Clone)]
pub enum XyChunk {
    TextDelta(String),
    ThinkingDelta(String),
    FunctionCall {
        name: String,
        args: Value,
        id: String,
    },
    Done {
        finish_reason: XyFinishReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyFinishReason {
    Stop,
    MaxTokens,
}

pub struct XyToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
