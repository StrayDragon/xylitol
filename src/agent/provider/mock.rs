use adk_core::{AdkError, Llm, LlmRequest, LlmResponse, LlmResponseStream};
use async_trait::async_trait;

/// Minimal mock LLM that always returns the same response.
/// Drop-in replacement for `adk_model::MockLlm` used in tests.
pub(crate) struct MockLlm {
    name: String,
    response: LlmResponse,
}

impl MockLlm {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            response: LlmResponse::default(),
        }
    }

    pub(crate) fn with_response(mut self, response: LlmResponse) -> Self {
        self.response = response;
        self
    }
}

#[async_trait]
impl Llm for MockLlm {
    fn name(&self) -> &str {
        &self.name
    }

    async fn generate_content(
        &self,
        _req: LlmRequest,
        _stream: bool,
    ) -> Result<LlmResponseStream, AdkError> {
        let response = self.response.clone();
        Ok(Box::pin(futures::stream::once(async move { Ok(response) })))
    }
}
