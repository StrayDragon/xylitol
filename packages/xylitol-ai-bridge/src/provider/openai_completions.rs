//! OpenAI Chat Completions adapter.
//!
//! HTTP via current transport (reqwest) + portable hook three-seam (c998).
//! Hooks use [`crate::hooks::HeaderBag`]; reqwest conversion stays
//! in private `reqwest_bridge`. Public `XyModel` surface is only
//! [`super::AdapterXyModel`] wrapping this [`crate::infra::provider::adapter::AiBridgeLlmAdapter`] (c505).

use std::sync::Arc;

use async_trait::async_trait;

use crate::dto::AiBridgeMessage;
use crate::dto::AiBridgeStream;
use crate::dto::AiBridgeToolSchema;
use crate::error::AiBridgeError;
use crate::hooks::HttpHooks;
use crate::provider::openai::OpenAIProvider;

use super::AiBridgeLlmAdapter;

/// Adapter for the OpenAI Chat Completions API.
pub struct OpenAiCompletionsAdapter {
    inner: OpenAIProvider,
}

impl OpenAiCompletionsAdapter {
    /// Create a new Chat Completions adapter.
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        Self {
            inner: OpenAIProvider::new(api_key, model, base_url, hooks),
        }
    }
}

#[async_trait]
impl AiBridgeLlmAdapter for OpenAiCompletionsAdapter {
    fn name(&self) -> &str {
        "openai-completions"
    }

    async fn generate_stream(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
    ) -> Result<AiBridgeStream, AiBridgeError> {
        self.inner.generate_stream(messages, tools, true).await
    }

    async fn generate(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
    ) -> Result<AiBridgeStream, AiBridgeError> {
        self.inner.generate_stream(messages, tools, false).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::StreamExt;
    use serde_json::Value;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::hooks::{HeaderBag, HttpHooks};

    struct NoopHooks;

    #[async_trait]
    impl HttpHooks for NoopHooks {
        async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
            Ok(())
        }

        async fn before_request(
            &self,
            _model: &str,
            _body: &mut Value,
        ) -> Result<(), AiBridgeError> {
            Ok(())
        }

        async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
    }

    struct ModifyBodyHooks;

    #[async_trait]
    impl HttpHooks for ModifyBodyHooks {
        async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
            Ok(())
        }

        async fn before_request(
            &self,
            _model: &str,
            body: &mut Value,
        ) -> Result<(), AiBridgeError> {
            body["cache_control"] = serde_json::json!({"type": "ephemeral"});
            Ok(())
        }

        async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
    }

    #[tokio::test]
    async fn empty_hooks_completions_noop() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "hi"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(NoopHooks)),
        );
        let mut stream = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[])
            .await
            .expect("generate");
        let mut saw_text = false;
        while let Some(item) = stream.next().await {
            let chunk = item.expect("chunk");
            if matches!(chunk, crate::dto::AiBridgeChunk::TextDelta(_)) {
                saw_text = true;
            }
        }
        assert!(saw_text);
    }

    #[tokio::test]
    async fn before_provider_request_modify_reaches_wiremock() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "cache_control": {"type": "ephemeral"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(ModifyBodyHooks)),
        );
        let _ = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[])
            .await
            .expect("generate with modify");
    }
}
