use crate::agent::model::ModelConfig;
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

// ── Thinking Level ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
}

impl ThinkingLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThinkingLevel::Off => "off",
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
        }
    }

    /// Clamp to what the model supports.
    pub fn clamp(self, model_supports_thinking: bool) -> Self {
        if !model_supports_thinking {
            return ThinkingLevel::Off;
        }
        self
    }
}

// ── Model Metadata ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ModelMeta {
    pub id: String,
    pub config: ModelConfig,
    pub display_name: String,
    pub thinking: bool,
    pub context_window: u64,
}
