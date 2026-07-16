//! MCP client manager — connects to MCP servers and dispatches tool calls.

use std::collections::HashMap;

use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, serve_client};
use serde_json::Value;
use tokio::process::Command;

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

/// Manages connections to MCP servers and dispatches tool calls.
pub struct McpClientManager {
    services: tokio::sync::Mutex<HashMap<String, McpService>>,
    transports: tokio::sync::Mutex<HashMap<String, McpTransportKind>>,
    diagnostics: tokio::sync::Mutex<Vec<McpConnectDiagnostic>>,
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            services: tokio::sync::Mutex::new(HashMap::new()),
            transports: tokio::sync::Mutex::new(HashMap::new()),
            diagnostics: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    /// Connect to all MCP servers from the app configuration.
    pub async fn connect(&self, config: &AppConfig) -> Result<(), String> {
        match &config.mcp_servers {
            Some(servers) if !servers.is_empty() => self.connect_servers(servers).await,
            _ => Ok(()),
        }
    }

    /// Connect to an explicit server list (empty = no-op).
    ///
    /// Invalid configs and connection failures are recorded as diagnostics and
    /// logged; remaining servers still attempt connect (mcp4).
    pub async fn connect_servers(&self, servers: &[McpServerConfig]) -> Result<(), String> {
        {
            let mut diags = self.diagnostics.lock().await;
            diags.clear();
        }
        for server_config in servers {
            let name = server_config.name.clone();
            if let Err(e) = server_config.validate() {
                log::warn!("MCP server {name}: invalid config: {e}");
                self.push_diagnostic(name, e).await;
                continue;
            }
            let result = match server_config.transport {
                McpTransportKind::Stdio => self.connect_stdio(&name, server_config).await,
                McpTransportKind::Sse => self.connect_sse(&name, server_config).await,
            };
            if let Err(e) = result {
                log::warn!("MCP server {name}: connection failed: {e}");
                self.push_diagnostic(name, e).await;
                // Continue connecting to remaining servers.
            }
        }
        Ok(())
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
    pub async fn connected_servers(&self) -> Vec<ConnectedMcpServer> {
        let services = self.services.lock().await;
        let transports = self.transports.lock().await;
        let mut out = Vec::new();
        for (id, service) in services.iter() {
            let tool_count = match service.list_all_tools().await {
                Ok(t) => t.len(),
                Err(e) => {
                    log::warn!("list_all_tools failed server_id={id} error={e}");
                    0
                }
            };
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
        let url = config
            .url
            .as_ref()
            .ok_or_else(|| "url is required for sse transport".to_string())?;
        let transport = rmcp::transport::StreamableHttpClientTransport::from_uri(url.to_string());
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
    pub async fn list_all_tools(&self) -> Vec<(String, String, String, Value)> {
        let mut result = Vec::new();
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
                    continue;
                }
            };
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_client_manager_new() {
        let manager = McpClientManager::new();
        let services = manager.services.blocking_lock();
        assert!(services.is_empty());
    }

    #[test]
    fn test_connect_with_no_servers() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = McpClientManager::new();
        let config = AppConfig::default();
        rt.block_on(manager.connect(&config)).unwrap();
        let services = rt.block_on(async { manager.services.lock().await });
        assert!(services.is_empty());
    }

    #[test]
    fn test_connect_with_empty_servers_list() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let manager = McpClientManager::new();
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
        let manager = McpClientManager::new();
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
        let manager = McpClientManager::new();
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
        let manager = McpClientManager::new();
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
}
