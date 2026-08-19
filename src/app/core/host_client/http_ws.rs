//! HTTP POST unary + WebSocket downlink (product TUI carrier).

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::protocol::RpcMessage;
use crate::protocol::RpcResult;

use super::{HostClient, HostClientError, MuxStream};

/// Product attach client. Unreachable Host is a hard error (no InProcess fallback).
#[derive(Clone)]
pub struct HttpWsClient {
    base_url: String,
    http: reqwest::Client,
    /// Shared across clones: one TUI = one writer lease.
    writer_token: Arc<Mutex<Option<String>>>,
}

impl HttpWsClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
            writer_token: Arc::new(Mutex::new(None)),
        }
    }

    fn api(&self, method: &str) -> String {
        format!("{}/api/{method}", self.base_url)
    }

    fn ws_mux_url(&self) -> String {
        let ws_base = self
            .base_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        format!("{ws_base}/api/events.mux")
    }
}

#[async_trait]
impl HostClient for HttpWsClient {
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError> {
        let rpc_id = uuid::Uuid::new_v4().to_string();
        let writer_token = self.writer_token.lock().ok().and_then(|g| g.clone());
        let body = RpcMessage::ClientRequest {
            rpc_id: rpc_id.clone(),
            method: method.to_string(),
            payload,
            writer_token,
        };
        let resp = self
            .http
            .post(self.api(method))
            .json(&body)
            .send()
            .await
            .map_err(|e| HostClientError::transport(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(HostClientError::transport(format!(
                "HTTP {} from {}",
                resp.status(),
                self.api(method)
            )));
        }
        let parsed: RpcMessage = resp
            .json()
            .await
            .map_err(|e| HostClientError::transport(e.to_string()))?;
        match parsed {
            RpcMessage::ServerResponse {
                rpc_id: echo,
                result,
            } => {
                if echo != rpc_id {
                    return Err(HostClientError::RpcIdMismatch {
                        sent: rpc_id,
                        got: echo,
                    });
                }
                if let Some(Value::Object(map)) = result.value.as_ref()
                    && let Some(token) = map.get("writerToken").and_then(|v| v.as_str())
                    && let Ok(mut slot) = self.writer_token.lock()
                {
                    *slot = Some(token.to_string());
                }
                Ok(result)
            }
            other => Err(HostClientError::UnexpectedEnvelope(format!("{other:?}"))),
        }
    }

    async fn respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError> {
        let body = RpcMessage::ClientResponse {
            rpc_id: rpc_id.to_string(),
            payload,
        };
        let resp = self
            .http
            .post(self.api("respond"))
            .json(&body)
            .send()
            .await
            .map_err(|e| HostClientError::transport(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(HostClientError::transport(format!(
                "HTTP {} from /api/respond",
                resp.status()
            )));
        }
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        let url = self.ws_mux_url();
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| HostClientError::transport(format!("WS connect {url}: {e}")))?;
        let (_writer, mut reader) = ws_stream.split();
        Ok(Box::pin(async_stream::stream! {
            while let Some(msg) = reader.next().await {
                match msg {
                    Ok(Message::Text(text)) => match serde_json::from_str::<RpcMessage>(&text) {
                        Ok(frame) => yield Ok(frame),
                        Err(e) => yield Err(HostClientError::transport(e.to_string())),
                    },
                    Ok(Message::Close(_)) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        }))
    }
}
