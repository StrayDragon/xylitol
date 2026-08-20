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

    /// ClientResponse (`POST /api/respond`). HTTP body is a carrier ack, not a second RpcMessage.
    async fn respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError>;

    /// WebSocket (or in-process) downlink. The socket MUST NOT send business uplinks.
    async fn mux(&self) -> Result<MuxStream, HostClientError>;
}
