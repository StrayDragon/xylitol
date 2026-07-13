//! Provider adapter layer — isolates vendor-specific LLM API shapes.
//!
//! Each adapter owns one vendor API dialect (OpenAI Responses, Anthropic Messages,
//! OpenAI Chat Completions, etc.) and converts it into the internal [`crate::domain::types::XyChunk`]
//! stream consumed by the agent loop.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::error::XyError;
use crate::domain::message::AgentMessage;
use crate::domain::model::XyModelKind;
use crate::domain::types::XyToolSchema;
use crate::runtime_protocol::XyStream;

pub mod anthropic_messages;
pub mod factory;
pub mod openai_completions;
pub mod openai_responses;
pub mod xy_model;

pub use anthropic_messages::AnthropicMessagesAdapter;
pub use openai_completions::OpenAiCompletionsAdapter;
pub use openai_responses::OpenAiResponsesAdapter;
pub use xy_model::AdapterXyModel;

/// Supported adapter kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    /// OpenAI Responses API (`/v1/responses`).
    OpenAiResponses,
    /// OpenAI Chat Completions API (`/v1/chat/completions`).
    OpenAiCompletions,
    /// Anthropic Messages API (`/v1/messages`).
    AnthropicMessages,
}

impl AdapterKind {
    /// Default adapter kind for a provider kind.
    pub fn default_for(kind: XyModelKind) -> Self {
        match kind {
            XyModelKind::OpenAi => AdapterKind::OpenAiResponses,
            XyModelKind::Anthropic => AdapterKind::AnthropicMessages,
            XyModelKind::Fake => AdapterKind::OpenAiResponses,
        }
    }

    /// Parse from a config string.
    pub fn from_config_str(s: &str) -> Option<Self> {
        match s {
            "openai-responses" => Some(Self::OpenAiResponses),
            "openai-completions" => Some(Self::OpenAiCompletions),
            "anthropic-messages" => Some(Self::AnthropicMessages),
            _ => None,
        }
    }
}

impl std::fmt::Display for AdapterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterKind::OpenAiResponses => write!(f, "openai-responses"),
            AdapterKind::OpenAiCompletions => write!(f, "openai-completions"),
            AdapterKind::AnthropicMessages => write!(f, "anthropic-messages"),
        }
    }
}

/// A vendor-specific adapter that converts a request into a stream of [`crate::domain::types::XyChunk`].
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Adapter name for diagnostics.
    fn name(&self) -> &str;

    /// Execute a streaming request and return a stream of internal chunks.
    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError>;

    /// Execute a non-streaming request and return chunks as a one-shot stream.
    async fn generate(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError>;
}

/// Shared handle to an adapter.
pub type AdapterRef = Arc<dyn LlmAdapter>;
