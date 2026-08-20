//! In-process envelope client (tests / c2304 conformance). Print still uses `XyInProcessDriver`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::protocol::{RpcMessage, RpcResult, is_unary_method};

use super::{HostClient, HostClientError, MuxStream};

const DOWNLINK_CAP: usize = 256;

/// Local host half of [`InProcessClient`].
#[async_trait]
pub trait InProcessHost: Send + Sync {
    async fn handle_unary(&self, method: &str, payload: Value) -> RpcResult;
    async fn handle_respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError>;
    fn subscribe_downlink(&self) -> broadcast::Receiver<RpcMessage>;
}

/// Echo unary payloads; unknown methods fail; downlink is a broadcast bus.
pub struct EchoHost {
    downlink: broadcast::Sender<RpcMessage>,
}

impl EchoHost {
    pub fn new() -> Arc<Self> {
        let (downlink, _) = broadcast::channel(DOWNLINK_CAP);
        Arc::new(Self { downlink })
    }

    pub fn push_downlink(&self, msg: RpcMessage) {
        let _ = self.downlink.send(msg);
    }
}

#[async_trait]
impl InProcessHost for EchoHost {
    async fn handle_unary(&self, method: &str, payload: Value) -> RpcResult {
        if !is_unary_method(method) {
            return RpcResult::error("unknown_method", method);
        }
        RpcResult::ok_value(payload)
    }

    async fn handle_respond(&self, _rpc_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    fn subscribe_downlink(&self) -> broadcast::Receiver<RpcMessage> {
        self.downlink.subscribe()
    }
}

/// In-process [`HostClient`] sharing the product envelope (not a silent TUI fallback).
pub struct InProcessClient {
    host: Arc<dyn InProcessHost>,
}

impl InProcessClient {
    pub fn new(host: Arc<dyn InProcessHost>) -> Self {
        Self { host }
    }

    pub fn echo() -> (Self, Arc<EchoHost>) {
        let host = EchoHost::new();
        (Self::new(host.clone()), host)
    }
}

#[async_trait]
impl HostClient for InProcessClient {
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError> {
        Ok(self.host.handle_unary(method, payload).await)
    }

    async fn respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError> {
        self.host.handle_respond(rpc_id, payload).await
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        let mut rx = self.host.subscribe_downlink();
        Ok(Box::pin(async_stream::stream! {
            loop {
                match rx.recv().await {
                    Ok(msg) => yield Ok(msg),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::core::host_client::HostClient;
    use crate::protocol::RpcMessage;
    use futures::StreamExt;

    #[tokio::test]
    async fn echo_unary_and_unknown_method() {
        let (client, _) = InProcessClient::echo();
        let ok = client
            .unary("prompt", serde_json::json!({"message": "hi"}))
            .await
            .unwrap();
        assert!(ok.ok);
        let err = client.unary("quit", serde_json::json!({})).await.unwrap();
        assert!(!err.ok);
        assert_eq!(err.error.as_ref().unwrap().code, "unknown_method");
    }

    #[tokio::test]
    async fn mux_delivers_server_request() {
        let (client, host) = InProcessClient::echo();
        let mut mux = client.mux().await.unwrap();
        host.push_downlink(RpcMessage::ServerRequest {
            rpc_id: "e1".into(),
            method: "session/event".into(),
            payload: serde_json::json!({"seq": 1}),
        });
        let frame = mux.next().await.unwrap().unwrap();
        match frame {
            RpcMessage::ServerRequest { method, .. } => assert_eq!(method, "session/event"),
            other => panic!("unexpected {other:?}"),
        }
    }
}
