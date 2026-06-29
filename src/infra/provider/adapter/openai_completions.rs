//! OpenAI Chat Completions adapter (placeholder).
//!
//! Full implementation is deferred to a follow-up change. For now this adapter
//! delegates to [`async_openai`] so the `LlmAdapter` trait is satisfied and the
//! factory can select it based on config.

use async_trait::async_trait;

use crate::domain::error::XyError;
use crate::domain::message::AgentMessage;
use crate::domain::types::XyToolSchema;
use crate::infra::provider::openai::OpenAIProvider;
use crate::runtime_protocol::{XyModel, XyStream};

use super::LlmAdapter;

/// Adapter for the OpenAI Chat Completions API.
pub struct OpenAiCompletionsAdapter {
    inner: OpenAIProvider,
}

impl OpenAiCompletionsAdapter {
    /// Create a new Chat Completions adapter.
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        Self {
            inner: OpenAIProvider::new(api_key, model, base_url),
        }
    }
}

#[async_trait]
impl LlmAdapter for OpenAiCompletionsAdapter {
    fn name(&self) -> &str {
        "openai-completions"
    }

    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError> {
        self.inner.generate_stream(messages, tools, true).await
    }

    async fn generate(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError> {
        self.inner.generate_stream(messages, tools, false).await
    }
}
