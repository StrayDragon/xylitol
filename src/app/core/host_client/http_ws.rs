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

/// c2425 mux liveness knobs.
const MUX_PING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(20);
/// Two silent ping windows mark the link half-open.
const MUX_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(40);

impl HttpWsClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest client with connect timeout"),
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

    /// One unary POST under a caller-chosen envelope `rpcId` (c2460).
    async fn post_unary(
        &self,
        rpc_id: &str,
        method: &str,
        payload: Value,
    ) -> Result<RpcResult, HostClientError> {
        let writer_token = self.writer_token.lock().ok().and_then(|g| g.clone());
        let body = RpcMessage::ClientRequest {
            rpc_id: rpc_id.to_string(),
            method: method.to_string(),
            payload,
            writer_token,
        };
        // c2425 program authority: every unary wait is bounded. Known long
        // ops (reload reinstalls shared resources) get a graded window.
        let bound = if method == "reload_runtime" || method == "reload" {
            std::time::Duration::from_secs(300)
        } else {
            std::time::Duration::from_secs(30)
        };
        let resp = tokio::time::timeout(bound, async {
            let resp = self
                .http
                .post(self.api(method))
                .json(&body)
                .send()
                .await?
                .error_for_status()?;
            resp.json::<RpcMessage>().await
        })
        .await
        .map_err(|_| {
            HostClientError::transport(format!(
                "unary {method} timed out after {}s",
                bound.as_secs()
            ))
        })?
        .map_err(|e| HostClientError::transport(format!("unary {method}: {e}")))?;
        match resp {
            RpcMessage::ServerResponse {
                rpc_id: echo,
                result,
            } => {
                if echo != rpc_id {
                    return Err(HostClientError::RpcIdMismatch {
                        sent: rpc_id.to_string(),
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
}

#[async_trait]
impl HostClient for HttpWsClient {
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError> {
        let rpc_id = uuid::Uuid::new_v4().to_string();
        self.post_unary(&rpc_id, method, payload).await
    }

    async fn unary_with_id(
        &self,
        rpc_id: &str,
        method: &str,
        payload: Value,
    ) -> Result<RpcResult, HostClientError> {
        self.post_unary(rpc_id, method, payload).await
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
        let (mut sink, mut reader) = ws_stream.split();

        // c2425 half-open detection: periodic pings keep the connection honest;
        // the read side fails after two silent windows so a silently dead link
        // surfaces as an error and the driver resync path can re-subscribe.
        tokio::spawn(async move {
            let mut ping = tokio::time::interval(MUX_PING_INTERVAL);
            ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            ping.tick().await; // first tick is immediate
            loop {
                ping.tick().await;
                use futures::SinkExt;
                if sink.send(Message::Ping(Default::default())).await.is_err() {
                    return;
                }
            }
        });

        Ok(Box::pin(async_stream::stream! {
            loop {
                let msg = match tokio::time::timeout(MUX_IDLE_TIMEOUT, reader.next()).await {
                    Ok(Some(m)) => m,
                    Ok(None) => break,
                    Err(_) => {
                        yield Err(HostClientError::transport(format!(
                            "mux idle: no frames within {}s",
                            MUX_IDLE_TIMEOUT.as_secs()
                        )));
                        break;
                    }
                };
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
