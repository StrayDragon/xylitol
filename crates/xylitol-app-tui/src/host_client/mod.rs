use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use thiserror::Error;
use xylitol_protocol::{Notification, RpcResult};

mod http_ws;
mod in_process;
pub(crate) mod json_rpc;

pub use http_ws::HttpWsClient;
pub use in_process::{InProcessClient, InProcessHost};

pub type MuxStream =
    Pin<Box<dyn Stream<Item = Result<Notification, HostClientError>> + Send + 'static>>;

#[derive(Debug, Error)]
pub enum HostClientError {
    #[error("host transport: {0}")]
    Transport(String),
    #[error("protocol mismatch: host speaks {got}, client expects {expected}")]
    ProtocolMismatch { got: u32, expected: u32 },
    #[error("wire format mismatch: host offers {formats:?}, client requires \"{expected}\"")]
    WireFormatMismatch {
        formats: Vec<String>,
        expected: &'static str,
    },
}

impl HostClientError {
    pub fn transport(msg: impl Into<String>) -> Self {
        Self::Transport(msg.into())
    }

    pub fn is_fatal_mismatch(&self) -> bool {
        matches!(
            self,
            Self::ProtocolMismatch { .. } | Self::WireFormatMismatch { .. }
        )
    }
}

#[async_trait]
pub trait HostClient: Send + Sync {
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError>;

    async fn respond(&self, call_id: &str, payload: Value) -> Result<(), HostClientError>;

    async fn mux(&self) -> Result<MuxStream, HostClientError>;

    async fn reset_mux(&self) -> Result<(), HostClientError> {
        let _ = self;
        Ok(())
    }
}
