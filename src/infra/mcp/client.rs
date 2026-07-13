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

/// Manages connections to MCP servers and dispatches tool calls.
pub struct McpClientManager {
    services: tokio::sync::Mutex<HashMap<String, McpService>>,
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
    pub async fn connect_servers(&self, servers: &[McpServerConfig]) -> Result<(), String> {
        for server_config in servers {
            let name = server_config.name.clone();
            let result = match server_config.transport {
                McpTransportKind::Stdio => self.connect_stdio(&name, server_config).await,
                McpTransportKind::Sse => self.connect_sse(&name, server_config).await,
            };
            if let Err(e) = result {
                tracing::warn!("MCP server {name}: connection failed: {e}");
                // Continue connecting to remaining servers.
            }
        }
        Ok(())
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
        tracing::info!(name, transport = "stdio", "MCP server connected");
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
        tracing::info!(name, transport = "sse", "MCP server connected");
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
                    tracing::warn!(server_id, error = %e, "list_all_tools failed");
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
                tracing::warn!(server = %name, error = %e, "MCP server shutdown error");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_client_manager_new() {
        let manager = McpClientManager::new();
        // Should not panic and be properly initialized.
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
}
