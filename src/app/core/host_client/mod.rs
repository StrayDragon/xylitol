//! Four-quadrant typed client: unary POST, respond POST, mux downlink.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use thiserror::Error;

use crate::protocol::{RpcMessage, RpcResult};

#[cfg(feature = "server")]
mod http_ws;
mod in_process;

#[cfg(feature = "server")]
pub use http_ws::HttpWsClient;
pub use in_process::InProcessClient;

/// Stream of downlink `ServerRequest` envelopes (and ignored carrier frames).
pub type MuxStream =
    Pin<Box<dyn Stream<Item = Result<RpcMessage, HostClientError>> + Send + 'static>>;

#[derive(Debug, Error)]
pub enum HostClientError {
    #[error("host transport: {0}")]
    Transport(String),
    #[error("rpcId mismatch: sent {sent}, got {got}")]
    RpcIdMismatch { sent: String, got: String },
    #[error("expected server-response, got {0}")]
    UnexpectedEnvelope(String),
    /// Mux handshake spoke a different protocol version (ath44/c2480). Fatal:
    /// the driver MUST NOT retry-loop against a version it cannot talk to.
    #[error("protocol mismatch: host speaks {got}, client expects {expected}")]
    ProtocolMismatch { got: u32, expected: u32 },
}

impl HostClientError {
    pub fn transport(msg: impl Into<String>) -> Self {
        Self::Transport(msg.into())
    }
}

/// One typed client, two carriers ([`InProcessClient`] / [`HttpWsClient`]).
#[async_trait]
pub trait HostClient: Send + Sync {
    /// ClientRequest → ServerResponse (`POST /api/<method>` or in-process).
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError>;

    /// Unary with a caller-chosen envelope `rpcId` — the idempotency admission
    /// key (c2460). Retries of one logical command MUST reuse the same id so
    /// the host replays the first result instead of executing twice. Default:
    /// ignore the id (carrier without a wire identity) and fall back to
    /// [`HostClient::unary`].
    async fn unary_with_id(
        &self,
        rpc_id: &str,
        method: &str,
        payload: Value,
    ) -> Result<RpcResult, HostClientError> {
        let _ = rpc_id;
        self.unary(method, payload).await
    }

    /// ClientResponse (`POST /api/respond`). HTTP body is a carrier ack, not a second RpcMessage.
    async fn respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError>;

    /// WebSocket (or in-process) downlink. The socket MUST NOT send business uplinks.
    async fn mux(&self) -> Result<MuxStream, HostClientError>;
}
