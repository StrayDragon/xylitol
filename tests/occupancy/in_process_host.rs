use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast;
use xylitol_app_tui::host_client::{HostClientError, InProcessClient, InProcessHost};
use xylitol_protocol::{RpcMessage, RpcResult};

use xylitol_serve::occupancy::{HostState, handle_unary};

pub struct HostStateHost {
    host: Arc<HostState>,
}

impl HostStateHost {
    pub fn new(host: Arc<HostState>) -> Arc<Self> {
        Arc::new(Self { host })
    }
}

#[async_trait]
impl InProcessHost for HostStateHost {
    async fn handle_unary(&self, method: &str, payload: Value) -> RpcResult {
        handle_unary(&self.host, None, method, payload).await
    }

    async fn handle_respond(&self, rpc_id: &str, payload: Value) -> Result<(), HostClientError> {
        let _ = self.host.respond(rpc_id, payload).await;
        Ok(())
    }

    fn subscribe_downlink(&self) -> broadcast::Receiver<RpcMessage> {
        self.host.in_process_downlink.subscribe()
    }
}

pub fn in_process_client(host: Arc<HostState>) -> InProcessClient {
    InProcessClient::new(HostStateHost::new(host))
}
