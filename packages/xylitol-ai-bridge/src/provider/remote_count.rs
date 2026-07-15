//! Remote token-count APIs (Anthropic count_tokens, OpenAI responses/input_tokens).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::dto::AiBridgeMessage;
use crate::error::AiBridgeError;
use crate::hooks::{HeaderBag, HttpHooks, run_before_headers, run_before_request};
use crate::provider::openai_responses::messages_to_responses_input;
use crate::provider::reqwest_bridge::{from_reqwest_headers, to_reqwest_headers};

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Port for RemoteCount so tests can inject a stub without HTTP.
#[async_trait]
pub trait RemoteCounter: Send + Sync {
    async fn count_tokens(&self, messages: &[AiBridgeMessage]) -> Result<u64, AiBridgeError>;
}

/// Anthropic `POST /v1/messages/count_tokens` client.
pub struct AnthropicRemoteCounter {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    hooks: Option<Arc<dyn HttpHooks>>,
}

impl AnthropicRemoteCounter {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".into()),
            hooks,
        }
    }
}

#[async_trait]
impl RemoteCounter for AnthropicRemoteCounter {
    async fn count_tokens(&self, messages: &[AiBridgeMessage]) -> Result<u64, AiBridgeError> {
        // Minimal body: map user/assistant text for count_tokens.
        let mut anth_msgs = Vec::new();
        for msg in messages {
            match msg {
                AiBridgeMessage::UserMessage { content, .. } => {
                    let text = content
                        .iter()
                        .filter_map(|p| p.as_text())
                        .collect::<Vec<_>>()
                        .join("\n");
                    anth_msgs.push(serde_json::json!({"role":"user","content": text}));
                }
                AiBridgeMessage::AssistantMessage { content, .. } => {
                    let text = content
                        .iter()
                        .filter_map(|p| p.as_text())
                        .collect::<Vec<_>>()
                        .join("\n");
                    anth_msgs.push(serde_json::json!({"role":"assistant","content": text}));
                }
                _ => {}
            }
        }
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": anth_msgs,
        });
        let mut headers = HeaderBag::new();
        headers.insert(
            "content-type".into(),
            Value::String("application/json".into()),
        );
        headers.insert("x-api-key".into(), Value::String(self.api_key.clone()));
        headers.insert(
            "anthropic-version".into(),
            Value::String(ANTHROPIC_VERSION.into()),
        );
        run_before_headers(&self.hooks, &mut headers).await?;
        run_before_request(&self.hooks, &self.model, &mut body).await?;

        let url = format!("{}/v1/messages/count_tokens", self.base_url);
        let response = self
            .client
            .post(&url)
            .headers(to_reqwest_headers(&headers))
            .json(&body)
            .send()
            .await
            .map_err(|e| AiBridgeError::Provider(anyhow::anyhow!("count_tokens request: {e}")))?;

        let status = response.status().as_u16();
        run_after_response_local(
            &self.hooks,
            status,
            &from_reqwest_headers(response.headers()),
        )
        .await;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AiBridgeError::Provider(anyhow::anyhow!(
                "count_tokens HTTP {status}: {text}"
            )));
        }
        let json: Value = response
            .json()
            .await
            .map_err(|e| AiBridgeError::Provider(anyhow::anyhow!("count_tokens parse: {e}")))?;
        json.get("input_tokens")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                AiBridgeError::Provider(anyhow::anyhow!("count_tokens missing input_tokens"))
            })
    }
}

/// OpenAI Responses `POST /v1/responses/input_tokens` client (c1060 / c1070 SDK).
pub struct OpenAiResponsesRemoteCounter {
    client: async_openai::Client<async_openai::config::OpenAIConfig>,
    model: String,
}

impl OpenAiResponsesRemoteCounter {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        let base = base_url.map(|b| crate::provider::openai_client::normalize_openai_v1_base(&b));
        Self {
            client: crate::provider::openai_client::build_openai_client(api_key, base, hooks),
            model,
        }
    }
}

#[async_trait]
impl RemoteCounter for OpenAiResponsesRemoteCounter {
    async fn count_tokens(&self, messages: &[AiBridgeMessage]) -> Result<u64, AiBridgeError> {
        let input = messages_to_responses_input(messages);
        let body = serde_json::json!({
            "model": self.model,
            "input": input,
        });
        // byot: tolerate partial mock / compatible JSON; hooks run in middleware.
        let json: Value = self
            .client
            .responses()
            .get_input_token_counts_byot(body)
            .await
            .map_err(|e| AiBridgeError::Provider(anyhow::anyhow!("input_tokens: {e}")))?;
        json.get("input_tokens")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                AiBridgeError::Provider(anyhow::anyhow!("input_tokens missing input_tokens field"))
            })
    }
}

async fn run_after_response_local(
    hooks: &Option<Arc<dyn HttpHooks>>,
    status: u16,
    headers: &HeaderBag,
) {
    crate::hooks::run_after_response(hooks, status, headers).await;
}

/// Test / injection stub.
pub struct StubRemoteCounter {
    pub tokens: u64,
    pub fail: bool,
}

#[async_trait]
impl RemoteCounter for StubRemoteCounter {
    async fn count_tokens(&self, _messages: &[AiBridgeMessage]) -> Result<u64, AiBridgeError> {
        if self.fail {
            Err(AiBridgeError::Provider(anyhow::anyhow!("stub remote fail")))
        } else {
            Ok(self.tokens)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounting::{EstimateContextOpts, estimate_context};
    use crate::dto::TokenProvenance;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn stub_returns_tokens() {
        let stub = StubRemoteCounter {
            tokens: 42,
            fail: false,
        };
        assert_eq!(stub.count_tokens(&[]).await.unwrap(), 42);
    }

    #[tokio::test]
    async fn stub_fail_is_err() {
        let stub = StubRemoteCounter {
            tokens: 0,
            fail: true,
        };
        assert!(stub.count_tokens(&[]).await.is_err());
    }

    #[tokio::test]
    async fn openai_input_tokens_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/responses/input_tokens"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "response.input_tokens",
                "input_tokens": 123
            })))
            .mount(&server)
            .await;

        let counter = OpenAiResponsesRemoteCounter::new(
            "sk-test".into(),
            "gpt-4o".into(),
            Some(server.uri()),
            None,
        );
        let msgs = vec![AiBridgeMessage::user("hello")];
        assert_eq!(counter.count_tokens(&msgs).await.unwrap(), 123);

        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: None,
                stop_reason: None,
                remote_count: Some(Box::new(|_| Some(123))),
                tokenizer_estimate: None,
                allow_remote: true,
            },
        );
        assert_eq!(est.provenance, TokenProvenance::RemoteCount);
        assert_eq!(est.tokens, 123);
    }

    #[tokio::test]
    async fn openai_input_tokens_http_fail_does_not_claim_remote() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/responses/input_tokens"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let counter = OpenAiResponsesRemoteCounter::new(
            "sk-test".into(),
            "gpt-4o".into(),
            Some(server.uri()),
            None,
        );
        let msgs = vec![AiBridgeMessage::user("hello")];
        assert!(counter.count_tokens(&msgs).await.is_err());

        // Caller must not inject tokens on failure → Heuristic (no local tokenizer here).
        let est = estimate_context(
            &msgs,
            EstimateContextOpts {
                last_usage: None,
                stop_reason: None,
                remote_count: None,
                tokenizer_estimate: None,
                allow_remote: true,
            },
        );
        assert_ne!(est.provenance, TokenProvenance::RemoteCount);
        assert_ne!(est.provenance, TokenProvenance::Api);
        assert_eq!(est.provenance, TokenProvenance::Heuristic);
    }
}
