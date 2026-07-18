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
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        self.inner
            .generate_stream(messages, tools, true, &options)
            .await
    }

    async fn generate(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        self.inner
            .generate_stream(messages, tools, false, &options)
            .await
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
            .generate(vec![AiBridgeMessage::user("hi")], &[], Default::default())
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
            .generate(vec![AiBridgeMessage::user("hi")], &[], Default::default())
            .await
            .expect("generate with modify");
    }

    #[tokio::test]
    async fn completions_body_includes_reasoning_effort_medium() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "reasoning_effort": "medium"
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
            Some(Arc::new(NoopHooks)),
        );
        let opts = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let _ = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[], opts)
            .await
            .expect("generate medium");
    }

    #[tokio::test]
    async fn completions_body_omits_effort_when_off() {
        use std::sync::{Arc as StdArc, Mutex};

        let captured: StdArc<Mutex<Option<Value>>> = StdArc::new(Mutex::new(None));
        let captured_hook = captured.clone();

        struct CaptureHooks {
            body: StdArc<Mutex<Option<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
                Ok(())
            }

            async fn before_request(
                &self,
                _model: &str,
                body: &mut Value,
            ) -> Result<(), AiBridgeError> {
                *self.body.lock().unwrap() = Some(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
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
            Some(Arc::new(CaptureHooks {
                body: captured_hook,
            })),
        );
        let _ = adapter
            .generate(
                vec![AiBridgeMessage::user("hi")],
                &[],
                crate::thinking::AiBridgeGenerateOptions::default(),
            )
            .await
            .expect("generate off");

        let body = captured.lock().unwrap().clone().expect("captured body");
        assert!(
            body.get("reasoning_effort").is_none(),
            "off must omit reasoning_effort, got {body}"
        );
    }

    #[tokio::test]
    async fn completions_body_map_override_high_to_max() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "reasoning_effort": "max"
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

        let mut map = std::collections::HashMap::new();
        map.insert("high".into(), Some("max".into()));
        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(NoopHooks)),
        );
        let opts = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "high".into(),
            level_map: map,
            thinking_budgets: None,
            system_prompt: None,
        };
        let _ = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[], opts)
            .await
            .expect("generate map override");
    }

    /// After switching level Off → high, the next Completions body MUST carry
    /// `reasoning_effort: "high"` (c1165 regression lock for UI cycle → request).
    #[tokio::test]
    async fn completions_body_effort_follows_level_switch() {
        use std::sync::{Arc as StdArc, Mutex};

        let captured: StdArc<Mutex<Vec<Value>>> = StdArc::new(Mutex::new(Vec::new()));
        let captured_hook = captured.clone();

        struct CaptureHooks {
            bodies: StdArc<Mutex<Vec<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
                Ok(())
            }

            async fn before_request(
                &self,
                _model: &str,
                body: &mut Value,
            ) -> Result<(), AiBridgeError> {
                self.bodies.lock().unwrap().push(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
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
            Some(Arc::new(CaptureHooks {
                bodies: captured_hook,
            })),
        );

        let _ = adapter
            .generate(
                vec![AiBridgeMessage::user("hi")],
                &[],
                crate::thinking::AiBridgeGenerateOptions {
                    thinking_level: "off".into(),
                    ..Default::default()
                },
            )
            .await
            .expect("off");
        let _ = adapter
            .generate(
                vec![AiBridgeMessage::user("hi")],
                &[],
                crate::thinking::AiBridgeGenerateOptions {
                    thinking_level: "high".into(),
                    ..Default::default()
                },
            )
            .await
            .expect("high after switch");

        let bodies = captured.lock().unwrap().clone();
        assert_eq!(bodies.len(), 2, "expected two requests");
        assert!(
            bodies[0].get("reasoning_effort").is_none(),
            "off: {bodies:?}"
        );
        assert_eq!(
            bodies[1].get("reasoning_effort"),
            Some(&serde_json::json!("high")),
            "after switch to high: {bodies:?}"
        );
    }
}
