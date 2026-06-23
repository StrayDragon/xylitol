//! Core data types — streaming chunks, thinking levels, model metadata, tool schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::message::Usage;
use crate::core::model::ModelConfig;

// ── Streaming Chunk ──────────────────────────────────────────────

/// A single chunk from a streaming LLM response.
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
        usage: Option<Usage>,
    },
}

/// Reason an LLM response finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyFinishReason {
    Stop,
    MaxTokens,
}

/// Alias for [`StopReason`](crate::core::message::StopReason).
pub use crate::core::message::StopReason as XyStopReason;

/// JSON schema describing a tool's parameters.
pub struct XyToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

// ── Thinking Level ──────────────────────────────────────────────────

/// How much "thinking" / chain-of-thought the model should expose.
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

/// Metadata describing a model variant available from a provider.
#[derive(Debug, Clone)]
pub struct ModelMeta {
    pub id: String,
    pub config: ModelConfig,
    pub display_name: String,
    pub thinking: bool,
    pub context_window: u64,
    pub api: String,
    pub provider: String,
    pub cost_input: f64,
    pub cost_output: f64,
    pub cost_cache_read: f64,
    pub cost_cache_write: f64,
    pub max_tokens: u64,
    pub thinking_levels: Vec<String>,
}
