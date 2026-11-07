use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use url::Url;
use xylitol_plugin_sdk::mcp::{McpOAuthEvent, McpOAuthGateway};

use crate::occupancy::host::SessionSlot;

struct PendingFlow {
    rx: Option<tokio::sync::oneshot::Receiver<String>>,
    task: tokio::task::JoinHandle<()>,
}

pub struct HostOAuthGateway {
    flows: Mutex<HashMap<String, PendingFlow>>,
    slot: Option<Arc<SessionSlot>>,
}

impl HostOAuthGateway {
    pub fn new() -> Self {
        Self {
            flows: Mutex::new(HashMap::new()),
            slot: None,
        }
    }

    pub fn with_slot(slot: Arc<SessionSlot>) -> Self {
        Self {
            flows: Mutex::new(HashMap::new()),
            slot: Some(slot),
        }
    }
}

impl Default for HostOAuthGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpOAuthGateway for HostOAuthGateway {
    async fn bind(&self, flow_id: &str, redirect_uri: Option<&str>) -> Result<String, String> {
        let (addr, expected_path, returned) = match redirect_uri {
            Some(raw) => {
                let uri = Url::parse(raw).map_err(|_| format!("invalid redirect_uri: {raw}"))?;
                let port = uri.port().unwrap_or(80);
                let path = uri.path().to_string();
                (format!("127.0.0.1:{port}"), path, raw.to_string())
            }
            None => (
                "127.0.0.1:0".to_string(),
                "/callback".to_string(),
                String::new(),
            ),
        };
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|error| format!("cannot bind OAuth callback listener {addr}: {error}"))?;
        let port = listener
            .local_addr()
            .map_err(|error| format!("cannot read OAuth callback port: {error}"))?
            .port();
        let redirect = if returned.is_empty() {
            format!("http://127.0.0.1:{port}/callback")
        } else {
            returned
        };
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut buf = [0u8; 8192];
            let Ok(read) = stream.read(&mut buf).await else {
                return;
            };
            let request = String::from_utf8_lossy(&buf[..read]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");
            if !path.starts_with(&expected_path) {
                let body = b"not found";
                let _ = stream
                    .write_all(
                        format!(
                            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await;
                let _ = stream.write_all(body).await;
                return;
            }
            let callback_url = format!("http://127.0.0.1:{port}{path}");
            let _ = tx.send(callback_url);
            let body = b"authorization received; you can close this window";
            let _ = stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await;
            let _ = stream.write_all(body).await;
        });
        self.flows
            .lock()
            .await
            .insert(flow_id.to_string(), PendingFlow { rx: Some(rx), task });
        Ok(redirect)
    }

    async fn wait(&self, flow_id: &str) -> Result<String, String> {
        let rx = {
            let mut flows = self.flows.lock().await;
            let Some(flow) = flows.get_mut(flow_id) else {
                return Err("OAuth flow not found".into());
            };
            flow.rx
                .take()
                .ok_or_else(|| "OAuth flow is already waiting".to_string())?
        };
        match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
            Ok(Ok(url)) => Ok(url),
            Ok(Err(_)) => Err("OAuth callback listener closed".into()),
            Err(_) => {
                self.cancel(flow_id).await;
                Err("OAuth callback timed out".into())
            }
        }
    }

    fn publish(&self, event: McpOAuthEvent) {
        let Some(slot) = self.slot.as_ref() else {
            return;
        };
        let method = match &event {
            McpOAuthEvent::Started { .. } => "mcp/oauth_started",
            McpOAuthEvent::Completed { .. } => "mcp/oauth_completed",
            McpOAuthEvent::Failed { .. } => "mcp/oauth_failed",
        };
        let params = serde_json::to_value(event).unwrap_or(serde_json::Value::Null);
        let _ = slot
            .in_process_downlink
            .send(xylitol_protocol::Notification {
                method: method.to_string(),
                params,
            });
    }

    async fn cancel(&self, flow_id: &str) {
        if let Some(flow) = self.flows.lock().await.remove(flow_id) {
            flow.task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn cancel_removes_flow() {
        let gateway = Arc::new(HostOAuthGateway::new());
        let _ = gateway.bind("flow-cancel", None).await.expect("bind");
        gateway.cancel("flow-cancel").await;
        assert!(gateway.wait("flow-cancel").await.is_err());
    }

    #[tokio::test]
    async fn callback_round_trip() {
        let gateway = Arc::new(HostOAuthGateway::new());
        let redirect = gateway.bind("flow-1", None).await.expect("bind");
        let uri = Url::parse(&redirect).expect("redirect uri");
        let port = uri.port().expect("port");
        let wait = tokio::spawn({
            let gateway = gateway.clone();
            async move { gateway.wait("flow-1").await }
        });
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        stream
            .write_all(b"GET /callback?code=abc&state=xyz&iss=https%3A%2F%2Fas.example HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .await
            .expect("write");
        let mut buf = [0u8; 512];
        let _ = stream.read(&mut buf).await;
        let callback = wait.await.expect("wait").expect("callback");
        assert!(callback.contains("code=abc"), "{callback}");
        assert!(callback.contains("state=xyz"), "{callback}");
    }
}
