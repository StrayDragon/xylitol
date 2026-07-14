//! OpenAI Chat Completions adapter.
//!
//! HTTP via the internal OpenAI HTTP client (async-openai). Public `XyModel` surface
//! is only [`super::AdapterXyModel`] wrapping this [`crate::infra::provider::adapter::LlmAdapter`] (c505).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::error::XyError;
use crate::domain::message::AgentMessage;
use crate::domain::types::XyToolSchema;
use crate::infra::hooks::HookDispatcher;
use crate::infra::provider::openai::OpenAIProvider;
use crate::runtime_protocol::XyStream;

use super::LlmAdapter;

/// Adapter for the OpenAI Chat Completions API.
pub struct OpenAiCompletionsAdapter {
    inner: OpenAIProvider,
    /// Provider script hooks — accepted for wiring parity; HTTP three-seam is
    /// intentionally no-op on Completions (Responses/Anthropic only).
    _hooks: Option<Arc<HookDispatcher>>,
}

impl OpenAiCompletionsAdapter {
    /// Create a new Chat Completions adapter.
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<HookDispatcher>>,
    ) -> Self {
        Self {
            inner: OpenAIProvider::new(api_key, model, base_url),
            _hooks: hooks,
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
