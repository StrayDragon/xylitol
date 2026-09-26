//! In-process envelope client (tests / c2304 conformance). Print still uses `XyInProcessDriver`.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::protocol::{RpcMessage, RpcResult, is_unary_method};

use super::{HostClient, HostClientError, MuxStream};

const DOWNLINK_CAP: usize = 256;

/// Local host half of [`InProcessClient`].
#[async_trait]
pub trait InProcessHost: Send + Sync {
    async fn handle_unary(
        &self,
        method: &str,
        payload: Value,
        writer_token: Option<String>,
    ) -> RpcResult;
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
    async fn handle_unary(
        &self,
        method: &str,
        payload: Value,
        _writer_token: Option<String>,
    ) -> RpcResult {
        if !is_unary_method(method) {
            return RpcResult::error("unknown_method", method);
        }
        if method == "host.describe" {
            return RpcResult::ok_value(serde_json::json!({
                "protocol": crate::protocol::wire::envelope::PROTOCOL_VERSION
            }));
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

#[cfg(feature = "server")]
/// Real in-process Host carrier used by conformance tests and embedded clients.
pub struct HostStateHost {
    host: Arc<crate::app::server::host::HostState>,
}

#[cfg(feature = "server")]
impl HostStateHost {
    pub fn new(host: Arc<crate::app::server::host::HostState>) -> Arc<Self> {
        Arc::new(Self { host })
    }
}

#[cfg(feature = "server")]
#[async_trait]
impl InProcessHost for HostStateHost {
    async fn handle_unary(
        &self,
        method: &str,
        payload: Value,
        writer_token: Option<String>,
    ) -> RpcResult {
        crate::app::server::host::handle_unary(&self.host, None, method, payload, writer_token)
            .await
    }

    async fn handle_respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError> {
        let _ = self.host.respond(rpc_id, payload).await;
        Ok(())
    }

    fn subscribe_downlink(&self) -> broadcast::Receiver<RpcMessage> {
        self.host.in_process_downlink.subscribe()
    }
}

/// In-process [`HostClient`] sharing the product envelope (not a silent TUI fallback).
#[derive(Clone)]
pub struct InProcessClient {
    host: Arc<dyn InProcessHost>,
    writer_token: Arc<Mutex<Option<String>>>,
}

impl InProcessClient {
    pub fn new(host: Arc<dyn InProcessHost>) -> Self {
        Self {
            host,
            writer_token: Arc::new(Mutex::new(None)),
        }
    }

    pub fn echo() -> (Self, Arc<EchoHost>) {
        let host = EchoHost::new();
        (Self::new(host.clone()), host)
    }

    #[cfg(feature = "server")]
    pub fn host_state(host: Arc<crate::app::server::host::HostState>) -> Self {
        Self::new(HostStateHost::new(host))
    }
}

#[async_trait]
impl HostClient for InProcessClient {
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError> {
        let writer_token = self
            .writer_token
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        let result = self.host.handle_unary(method, payload, writer_token).await;
        if let Some(Value::Object(map)) = result.value.as_ref()
            && let Some(token) = map.get("writerToken").and_then(Value::as_str)
            && let Ok(mut guard) = self.writer_token.lock()
        {
            *guard = Some(token.to_string());
        }
        Ok(result)
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

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn host_state_carrier_preserves_writer_lease_and_read_only_access() {
        let host = crate::app::server::host::HostState::for_test().expect("host");
        let client = InProcessClient::host_state(host.clone());
        let other_client = InProcessClient::host_state(host);

        let read_only = other_client
            .unary("list_sessions", serde_json::json!({}))
            .await
            .expect("list sessions");
        assert!(read_only.ok);

        let acquired = client
            .unary(
                "set_session_name",
                serde_json::json!({
                    "session_id": "carrier-session",
                    "name": "carrier",
                }),
            )
            .await
            .expect("set name");
        assert!(acquired.ok);
        assert!(
            acquired
                .value
                .as_ref()
                .and_then(|value| value.get("writerToken"))
                .and_then(Value::as_str)
                .is_some()
        );

        let conflict = other_client
            .unary(
                "set_session_name",
                serde_json::json!({
                    "session_id": "carrier-session",
                    "name": "blocked",
                }),
            )
            .await
            .expect("writer conflict response");
        assert!(!conflict.ok);
        assert_eq!(
            conflict.error.as_ref().map(|error| error.code.as_str()),
            Some("writer_conflict")
        );

        let unknown = client
            .unary("not_a_method", serde_json::json!({}))
            .await
            .expect("unknown method response");
        assert!(!unknown.ok);
        assert_eq!(
            unknown.error.as_ref().map(|error| error.code.as_str()),
            Some("not_found")
        );
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn host_state_abort_cancels_process_reload_without_writer_lease() {
        let host = crate::app::server::host::HostState::for_test().expect("host");
        let cancel = tokio_util::sync::CancellationToken::new();
        *host.reload_cancel.lock().await = Some(cancel.clone());

        let client = InProcessClient::host_state(host);
        let result = client
            .unary("abort", serde_json::json!({}))
            .await
            .expect("abort response");

        assert!(result.ok);
        assert_eq!(
            result
                .value
                .as_ref()
                .and_then(|value| value.get("cancelled"))
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(cancel.is_cancelled());
    }
}
