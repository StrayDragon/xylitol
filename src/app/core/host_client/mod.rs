//! Typed host client: product unary + mux downlink (in-process or WS `/rpc`).

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
mod wire_v3_client;

pub use http_ws::dual_rail_event_parity;

#[cfg(feature = "server")]
pub use http_ws::HttpWsClient;
pub use in_process::InProcessClient;
#[cfg(feature = "server")]
pub use wire_v3_client::{client_request_bytes, server_response_to_result, stable_rpc_id};

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
    /// Product unary (`POST /rpc` / `WS /rpc` v3, or in-process).
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

    /// Approve / answer via product unary (`approve_tool` / `answer_question`).
    async fn respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError>;

    /// WebSocket (or in-process) downlink; WS `/rpc` accepts v3 binary unary.
    async fn mux(&self) -> Result<MuxStream, HostClientError>;

    /// Drop a cached mux transport so the next [`Self::mux`] opens a fresh
    /// connection. HTTP/WS MUST re-enter Host `unbound_mux` after SwitchSession
    /// / NewSession; in-process broadcast is a no-op.
    async fn reset_mux(&self) -> Result<(), HostClientError> {
        let _ = self;
        Ok(())
    }
}
