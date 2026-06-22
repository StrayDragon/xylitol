//! Re-exports and auxiliary agent types (streaming, thinking, model metadata).
//!
//! Message types are in [`crate::agent::message`].

use crate::agent::model::ModelConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Re-export new message types for convenience.
pub use crate::agent::message::{
    AgentMessage, AgentPart, ImageContent, StopReason, Usage,
};

// ── Streaming Chunk ──────────────────────────────────────────────

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
        /// Token usage from the provider response, if available.
        usage: Option<crate::agent::message::Usage>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XyFinishReason {
    Stop,
    MaxTokens,
}

/// Alias for the new [`StopReason`] — mirrors `XyFinishReason` for provider
/// code that hasn't been migrated yet.
pub use crate::agent::message::StopReason as XyStopReason;

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
    /// Provider API name (e.g. "openai", "anthropic").
    pub api: String,
    /// Provider name (e.g. "openai", "anthropic", "deepseek").
    pub provider: String,
    /// Cost per million input tokens in USD.
    pub cost_input: f64,
    /// Cost per million output tokens in USD.
    pub cost_output: f64,
    /// Cost per million cache-read tokens in USD.
    pub cost_cache_read: f64,
    /// Cost per million cache-write tokens in USD.
    pub cost_cache_write: f64,
    /// Maximum output tokens.
    pub max_tokens: u64,
    /// Supported thinking levels (e.g. ["off", "low", "medium", "high"]).
    pub thinking_levels: Vec<String>,
}
