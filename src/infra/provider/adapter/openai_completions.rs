//! OpenAI Chat Completions adapter.
//!
//! HTTP via current transport (reqwest) + portable hook three-seam (c998).
//! Hooks use [`crate::infra::hooks::http::HeaderBag`]; reqwest conversion is
//! [`crate::infra::provider::reqwest_bridge`] only. Public `XyModel` surface
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
            inner: OpenAIProvider::new(api_key, model, base_url, hooks),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::config::types::{HookEntry, HooksConfig};
    use futures::StreamExt;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
            Some(Arc::new(HookDispatcher::new(&HooksConfig::default()))),
        );
        let mut stream = adapter
            .generate(vec![AgentMessage::user("hi")], &[])
            .await
            .expect("generate");
        let mut saw_text = false;
        while let Some(item) = stream.next().await {
            let chunk = item.expect("chunk");
            if matches!(chunk, crate::domain::types::XyChunk::TextDelta(_)) {
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

        let config = HooksConfig {
            global: vec![HookEntry {
                events: vec!["before_provider_request".into()],
                command: "echo '{\"action\":\"modify\",\"args\":{\"model\":\"gpt-test\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"cache_control\":{\"type\":\"ephemeral\"}}}'".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(HookDispatcher::new(&config))),
        );
        let _ = adapter
            .generate(vec![AgentMessage::user("hi")], &[])
            .await
            .expect("generate with modify");
    }
}
