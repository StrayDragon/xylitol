use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast;

use xylitol_protocol::registry::METHOD_HOST_DESCRIBE;
use xylitol_protocol::{
    Notification, PROTOCOL_VERSION, RpcResult, WIRE_FORMAT_JSONRPC, is_unary_method,
};

use super::{HostClient, HostClientError, MuxStream};

const DOWNLINK_CAP: usize = 256;

#[async_trait]
pub trait InProcessHost: Send + Sync {
    async fn handle_unary(&self, method: &str, payload: Value) -> RpcResult;
    async fn handle_respond(&self, call_id: &str, payload: Value) -> Result<(), HostClientError>;
    fn subscribe_downlink(&self) -> broadcast::Receiver<Notification>;
}

pub struct EchoHost {
    downlink: broadcast::Sender<Notification>,
}

impl EchoHost {
    pub fn new() -> Arc<Self> {
        let (downlink, _) = broadcast::channel(DOWNLINK_CAP);
        Arc::new(Self { downlink })
    }

    pub fn push_downlink(&self, note: Notification) {
        let _ = self.downlink.send(note);
    }
}

#[async_trait]
impl InProcessHost for EchoHost {
    async fn handle_unary(&self, method: &str, payload: Value) -> RpcResult {
        if !is_unary_method(method) {
            return RpcResult::error("unregistered_method", method);
        }
        if method == METHOD_HOST_DESCRIBE {
            return RpcResult::ok_value(serde_json::json!({
                "protocol": PROTOCOL_VERSION,
                "formats": [WIRE_FORMAT_JSONRPC],
            }));
        }
        RpcResult::ok_value(payload)
    }

    async fn handle_respond(&self, _call_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    fn subscribe_downlink(&self) -> broadcast::Receiver<Notification> {
        self.downlink.subscribe()
    }
}

#[derive(Clone)]
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
    use crate::host_client::HostClient;
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
        assert_eq!(err.error.as_ref().unwrap().code, "unregistered_method");
    }

    #[tokio::test]
    async fn echo_describe_advertises_protocol_and_jsonrpc_format() {
        let (client, _) = InProcessClient::echo();
        let described = client
            .unary("host.describe", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(
            described.value,
            Some(serde_json::json!({
                "protocol": PROTOCOL_VERSION,
                "formats": [WIRE_FORMAT_JSONRPC],
            }))
        );
    }

    #[tokio::test]
    async fn mux_delivers_downlink_notification() {
        let (client, host) = InProcessClient::echo();
        let mut mux = client.mux().await.unwrap();
        host.push_downlink(Notification {
            method: "session/event".into(),
            params: serde_json::json!({"seq": 1}),
        });
        let frame = mux.next().await.unwrap().unwrap();
        assert_eq!(frame.method, "session/event");
        assert_eq!(frame.params, serde_json::json!({"seq": 1}));
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
        assert!(acquired.writer_token.is_none());

        let second = other_client
            .unary(
                "set_session_name",
                serde_json::json!({
                    "session_id": "carrier-session",
                    "name": "blocked",
                }),
            )
            .await
            .expect("same-process second client");
        assert!(
            second.ok,
            "same Host two clients share occupancy: {second:?}"
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
