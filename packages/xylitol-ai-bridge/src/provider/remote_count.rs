//! Remote token-count APIs (Anthropic count_tokens) with injectable stub.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::dto::AiBridgeMessage;
use crate::error::AiBridgeError;
use crate::hooks::{HeaderBag, HttpHooks, run_before_headers, run_before_request};
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
}
