#![allow(unused_imports)]

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use thiserror::Error;

use xylitol_protocol::{RpcMessage, RpcResult};

mod envelope_client;
mod http_ws;
mod in_process;

pub use envelope_client::{
    ENVELOPE_FORMAT_ENV, client_request_bytes, server_response_to_result, stable_rpc_id,
};
pub use http_ws::HttpWsClient;
pub use in_process::{InProcessClient, InProcessHost};

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
    #[error("protocol mismatch: host speaks {got}, client expects {expected}")]
    ProtocolMismatch { got: u32, expected: u32 },
}

impl HostClientError {
    pub fn transport(msg: impl Into<String>) -> Self {
        Self::Transport(msg.into())
    }
}

#[async_trait]
pub trait HostClient: Send + Sync {
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError>;

    async fn unary_with_id(
        &self,
        rpc_id: &str,
        method: &str,
        payload: Value,
    ) -> Result<RpcResult, HostClientError> {
        let _ = rpc_id;
        self.unary(method, payload).await
    }

    async fn respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError>;

    async fn mux(&self) -> Result<MuxStream, HostClientError>;

    async fn reset_mux(&self) -> Result<(), HostClientError> {
        let _ = self;
        Ok(())
    }
}
