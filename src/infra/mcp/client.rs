//! MCP client manager — connects to MCP servers and dispatches tool calls.

use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::{FuturesUnordered, StreamExt};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, serve_client};
use serde_json::Value;
use tokio::process::Command;
use tokio::sync::Mutex;

use super::types::{McpServerConfig, McpTransportKind};
use crate::infra::config::types::AppConfig;

type McpService = RunningService<RoleClient, ()>;

/// Diagnostic from validate / connect (c1080 / mcp4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectDiagnostic {
    pub server: String,
    pub message: String,
}

/// Read-only snapshot of a successfully connected MCP server (c1080 / mcp5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedMcpServer {
    pub id: String,
    pub transport: McpTransportKind,
    pub tool_count: usize,
}

/// Live connect progress for loaded-resources / gates (c1200).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpConnectProgress {
    /// True while [`McpClientManager::connect_servers`] is in flight.
    pub connecting: bool,
    pub total: usize,
    pub finished: usize,
    pub current: Option<String>,
}

impl McpConnectProgress {
    /// Short label for the mcp header row (`connecting 0/2`, `1/2`, …).
    ///
    /// Numerator is **ready/finished** count. Do **not** append a server id: with
    /// parallel connect the last-finished name reads like "currently connecting".
    pub fn connecting_label(&self) -> Option<String> {
        if !self.connecting || self.total == 0 {
            return None;
        }
        let n = self.finished.min(self.total);
        Some(format!("connecting {n}/{}", self.total))
    }
}

/// Manages connections to MCP servers and dispatches tool calls.
pub struct McpClientManager {
    services: Mutex<HashMap<String, McpService>>,
    transports: Mutex<HashMap<String, McpTransportKind>>,
    /// Tool counts from the last successful [`Self::list_all_tools`] (or connect-time list).
    /// `connected_servers` MUST use this cache — MUST NOT re-RPC `list_all_tools` (TUI hitch).
    tool_counts: Mutex<HashMap<String, usize>>,
    diagnostics: Mutex<Vec<McpConnectDiagnostic>>,
    progress: Mutex<McpConnectProgress>,
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            services: Mutex::new(HashMap::new()),
            transports: Mutex::new(HashMap::new()),
            tool_counts: Mutex::new(HashMap::new()),
            diagnostics: Mutex::new(Vec::new()),
            progress: Mutex::new(McpConnectProgress::default()),
        }
    }

    /// Connect to all MCP servers from the app configuration.
    pub async fn connect(self: &Arc<Self>, config: &AppConfig) -> Result<(), String> {
        match &config.mcp_servers {
            Some(servers) if !servers.is_empty() => self.connect_servers(servers).await,
            _ => Ok(()),
        }
    }

    /// Connect to an explicit server list (empty = no-op). Parallel per server (c1200).
    ///
    /// Invalid configs and connection failures are recorded as diagnostics and
    /// logged; remaining servers still attempt connect (mcp4).
    pub async fn connect_servers(
        self: &Arc<Self>,
        servers: &[McpServerConfig],
    ) -> Result<(), String> {
        self.connect_servers_with_progress(servers, None).await
    }

    /// Like [`Self::connect_servers`], optionally mirroring progress into `progress_out`.
    pub async fn connect_servers_with_progress(
        self: &Arc<Self>,
        servers: &[McpServerConfig],
        progress_out: Option<Arc<Mutex<McpConnectProgress>>>,
    ) -> Result<(), String> {
        {
            let mut diags = self.diagnostics.lock().await;
            diags.clear();
        }

        let mut validated = Vec::new();
        for server_config in servers {
            let name = server_config.name.clone();
            if let Err(e) = server_config.validate() {
                log::warn!("MCP server {name}: invalid config: {e}");
                self.push_diagnostic(name, e).await;
                continue;
            }
            validated.push(server_config.clone());
        }

        let total = validated.len();
        self.write_progress(&progress_out, true, total, 0, None)
            .await;

        let mut futs = FuturesUnordered::new();
        for server_config in validated {
            let this = Arc::clone(self);
            futs.push(async move {
                let name = server_config.name.clone();
                let result = match server_config.transport {
                    McpTransportKind::Stdio => this.connect_stdio(&name, &server_config).await,
                    McpTransportKind::Sse => this.connect_sse(&name, &server_config).await,
                };
                (name, result)
            });
        }

        let mut finished = 0usize;
        while let Some((name, result)) = futs.next().await {
            if let Err(e) = result {
                log::warn!("MCP server {name}: connection failed: {e}");
                self.push_diagnostic(name.clone(), e).await;
            }
            finished += 1;
            self.write_progress(&progress_out, true, total, finished, Some(name))
                .await;
        }

        // When a shared progress Arc is owned by bootstrap, leave `connecting=true`
        // until discover + manager merge (caller clears). Clearing here flashes
        // "N configured · 0 connected" while McpBootState is still Running.
        let clear = progress_out.is_none();
        self.write_progress(&progress_out, !clear, total, finished, None)
            .await;
        Ok(())
    }

    async fn write_progress(
        &self,
        progress_out: &Option<Arc<Mutex<McpConnectProgress>>>,
        connecting: bool,
        total: usize,
        finished: usize,
        current: Option<String>,
    ) {
        let snap = McpConnectProgress {
            connecting,
            total,
            finished,
            current,
        };
        *self.progress.lock().await = snap.clone();
        if let Some(out) = progress_out {
            *out.lock().await = snap;
        }
    }

    /// Latest connect progress (c1200).
    pub async fn progress_snapshot(&self) -> McpConnectProgress {
        self.progress.lock().await.clone()
    }
    async fn push_diagnostic(&self, server: String, message: String) {
        self.diagnostics
            .lock()
            .await
            .push(McpConnectDiagnostic { server, message });
    }

    /// Diagnostics from the last [`Self::connect_servers`] (validate + connect failures).
    pub async fn diagnostics(&self) -> Vec<McpConnectDiagnostic> {
        self.diagnostics.lock().await.clone()
    }

    /// Connected servers with tool counts (mcp5). Does not create new connections.
    ///
    /// Uses cached counts from the last [`Self::list_all_tools`] (discover / reload).
    /// MUST NOT call per-server `list_all_tools` RPC here — that blocked the TUI tick
    /// loop for hundreds of ms when the welcome card flipped to `N connected`.
    pub async fn connected_servers(&self) -> Vec<ConnectedMcpServer> {
        let services = self.services.lock().await;
        let transports = self.transports.lock().await;
        let counts = self.tool_counts.lock().await;
        let mut out = Vec::new();
        for id in services.keys() {
            let tool_count = counts.get(id).copied().unwrap_or(0);
            let transport = transports
                .get(id)
                .copied()
                .unwrap_or(McpTransportKind::Stdio);
            out.push(ConnectedMcpServer {
                id: id.clone(),
                transport,
                tool_count,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    async fn connect_stdio(&self, name: &str, config: &McpServerConfig) -> Result<(), String> {
        let command = config
            .command
            .as_ref()
            .ok_or_else(|| "command is required for stdio transport".to_string())?;
        let args = config.args.as_deref().unwrap_or_default();

        let mut cmd = Command::new(command);
        cmd.args(args);
        if let Some(ref env) = config.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        let transport = TokioChildProcess::new(cmd).map_err(|e| format!("spawn failed: {e}"))?;

        let service = serve_client((), transport)
            .await
            .map_err(|e| format!("init failed: {e}"))?;

        let mut services = self.services.lock().await;
        services.insert(name.to_string(), service);
        self.transports
            .lock()
            .await
            .insert(name.to_string(), McpTransportKind::Stdio);
        log::info!(
            "MCP server connected name={} transport={}",
            { name },
            "stdio"
        );
        Ok(())
    }

    async fn connect_sse(&self, name: &str, config: &McpServerConfig) -> Result<(), String> {
        use http::{HeaderName, HeaderValue};
        use rmcp::transport::StreamableHttpClientTransport;
        use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

        let url = config
            .url
            .as_ref()
            .ok_or_else(|| "url is required for sse transport".to_string())?;
        let mut transport_cfg = StreamableHttpClientTransportConfig::with_uri(url.clone());
        if let Some(ref headers) = config.headers {
            let mut custom = HashMap::new();
            for (k, v) in headers {
                let name = HeaderName::from_bytes(k.as_bytes())
                    .map_err(|e| format!("invalid header name {k:?}: {e}"))?;
                let value = HeaderValue::from_str(v)
                    .map_err(|e| format!("invalid header value for {k}: {e}"))?;
                custom.insert(name, value);
            }
            transport_cfg = transport_cfg.custom_headers(custom);
        }
        let transport = StreamableHttpClientTransport::from_config(transport_cfg);
        let service = serve_client((), transport)
            .await
            .map_err(|e| format!("init failed: {e}"))?;

        let mut services = self.services.lock().await;
        services.insert(name.to_string(), service);
        self.transports
            .lock()
            .await
            .insert(name.to_string(), McpTransportKind::Sse);
        log::info!("MCP server connected name={} transport={}", { name }, "sse");
        Ok(())
    }

    /// List all tools from all connected MCP servers.
    ///
    /// Returns `(server_id, tool_name, description, input_schema)` for each tool.
    /// Also refreshes [`Self::connected_servers`] tool-count cache.
    pub async fn list_all_tools(&self) -> Vec<(String, String, String, Value)> {
        let mut result = Vec::new();
        let mut counts: HashMap<String, usize> = HashMap::new();
        let services = self.services.lock().await;
        for (server_id, service) in services.iter() {
            let tools = match service.list_all_tools().await {
                Ok(t) => t,
                Err(e) => {
                    log::warn!(
                        "list_all_tools failed server_id={} error={}",
                        { server_id },
                        e
                    );
                    counts.insert(server_id.clone(), 0);
                    continue;
                }
            };
            counts.insert(server_id.clone(), tools.len());
            for tool in tools {
                let description = tool.description.as_deref().unwrap_or("").to_string();
                let schema = Value::Object(tool.input_schema.as_ref().clone());
                result.push((
                    server_id.clone(),
                    tool.name.to_string(),
                    description,
                    schema,
                ));
            }
        }
        drop(services);
        *self.tool_counts.lock().await = counts;
        result
    }

    /// Call a tool on a specific MCP server.
    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        args: Value,
    ) -> Result<Value, String> {
        let services = self.services.lock().await;
        let service = services
            .get(server_id)
            .ok_or_else(|| format!("MCP server not found: {server_id}"))?;

        let args_map = args.as_object().cloned().unwrap_or_default();
        let params = CallToolRequestParams::new(tool_name.to_string()).with_arguments(args_map);

        let result: CallToolResult = service
            .call_tool(params)
            .await
            .map_err(|e| format!("call {server_id}/{tool_name} failed: {e}"))?;

        let json = serde_json::to_value(&result).map_err(|e| format!("serialize result: {e}"))?;
        Ok(json)
    }

    /// Gracefully shut down all MCP connections.
    pub async fn shutdown(&self) {
        let mut services = self.services.lock().await;
        for (name, mut service) in services.drain() {
            if let Err(e) = service.close().await {
                log::warn!("MCP server shutdown error server={} error={}", name, e);
            }
        }
        self.transports.lock().await.clear();
        self.tool_counts.lock().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_mcp_client_manager_new() {
        let manager = McpClientManager::new();
        let services = manager.services.blocking_lock();
        assert!(services.is_empty());
    }

    #[test]
    fn test_connect_with_no_servers() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = Arc::new(McpClientManager::new());
        let config = AppConfig::default();
        rt.block_on(manager.connect(&config)).unwrap();
        let services = rt.block_on(async { manager.services.lock().await });
        assert!(services.is_empty());
    }

    #[test]
    fn test_connect_with_empty_servers_list() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = Arc::new(McpClientManager::new());
        let config = AppConfig {
            mcp_servers: Some(vec![]),
            ..Default::default()
        };
        rt.block_on(manager.connect(&config)).unwrap();
        let services = rt.block_on(async { manager.services.lock().await });
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn invalid_stdio_records_diagnostic_no_service() {
        let manager = Arc::new(McpClientManager::new());
        let servers = vec![McpServerConfig {
            name: "bad".into(),
            transport: McpTransportKind::Stdio,
            command: None,
            ..Default::default()
        }];
        manager.connect_servers(&servers).await.unwrap();
        let diags = manager.diagnostics().await;
        assert!(
            diags
                .iter()
                .any(|d| d.server == "bad" && d.message.contains("command")),
            "expected command diagnostic; got {diags:?}"
        );
        assert!(manager.connected_servers().await.is_empty());
    }

    #[tokio::test]
    async fn invalid_sse_url_records_diagnostic() {
        let manager = Arc::new(McpClientManager::new());
        let servers = vec![McpServerConfig {
            name: "bad-url".into(),
            transport: McpTransportKind::Sse,
            url: Some("not-a-url".into()),
            ..Default::default()
        }];
        manager.connect_servers(&servers).await.unwrap();
        let diags = manager.diagnostics().await;
        assert!(
            diags
                .iter()
                .any(|d| d.server == "bad-url" && d.message.contains("url")),
            "expected url diagnostic; got {diags:?}"
        );
    }

    #[tokio::test]
    async fn connect_fail_continues_and_ok() {
        let manager = Arc::new(McpClientManager::new());
        let servers = vec![
            McpServerConfig {
                name: "missing-bin".into(),
                transport: McpTransportKind::Stdio,
                command: Some("/nonexistent/xylitol-mcp-fake-bin".into()),
                ..Default::default()
            },
            McpServerConfig {
                name: "also-bad".into(),
                transport: McpTransportKind::Stdio,
                command: None,
                ..Default::default()
            },
        ];
        // Must not Err the whole batch.
        manager.connect_servers(&servers).await.unwrap();
        let diags = manager.diagnostics().await;
        assert!(diags.len() >= 2, "both failures observable; got {diags:?}");
        assert!(manager.connected_servers().await.is_empty());
    }

    #[tokio::test]
    async fn connected_servers_empty_without_connect() {
        let manager = McpClientManager::new();
        assert!(manager.connected_servers().await.is_empty());
        assert!(manager.diagnostics().await.is_empty());
    }

    #[tokio::test]
    async fn connected_servers_reads_cached_counts_without_relisting() {
        // After discover, list_all_tools fills tool_counts; connected_servers must
        // not re-issue MCP list RPCs (would hitch the TUI welcome-card refresh).
        let manager = Arc::new(McpClientManager::new());
        let _ = manager.list_all_tools().await;
        let t0 = std::time::Instant::now();
        let rows = manager.connected_servers().await;
        assert!(t0.elapsed().as_millis() < 50, "cache path must stay local");
        assert!(rows.is_empty());
    }

    #[test]
    fn connecting_label_is_ready_over_total() {
        let start = McpConnectProgress {
            connecting: true,
            total: 2,
            finished: 0,
            current: None,
        };
        assert_eq!(start.connecting_label().as_deref(), Some("connecting 0/2"));

        let mid = McpConnectProgress {
            connecting: true,
            total: 2,
            finished: 1,
            current: Some("lspz".into()),
        };
        assert_eq!(mid.connecting_label().as_deref(), Some("connecting 1/2"));

        let done_but_boot = McpConnectProgress {
            connecting: true,
            total: 2,
            finished: 2,
            current: None,
        };
        assert_eq!(
            done_but_boot.connecting_label().as_deref(),
            Some("connecting 2/2")
        );

        let settled = McpConnectProgress {
            connecting: false,
            total: 2,
            finished: 2,
            current: None,
        };
        assert!(settled.connecting_label().is_none());
    }
}
