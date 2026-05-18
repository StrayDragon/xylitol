//! MCP (Model Context Protocol) client integration.
//!
//! Provides [`McpClientManager`] to connect to MCP servers via stdio or SSE
//! transport, and [`McpToolAdapter`] to expose MCP tools as [`adk_core::Tool`].

use std::collections::HashMap;
use std::sync::Arc;

use adk_core::{AdkError, ErrorCategory, ErrorComponent};
use async_trait::async_trait;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::RunningService;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::{RoleClient, serve_client};
use serde_json::Value;
use tokio::process::Command;

use crate::infra::config::types::{AppConfig, McpTransportKind};

type McpService = RunningService<RoleClient, ()>;

/// Manages connections to MCP servers and dispatches tool calls.
pub(crate) struct McpClientManager {
    services: tokio::sync::Mutex<HashMap<String, McpService>>,
}

impl McpClientManager {
    pub(crate) fn new() -> Self {
        Self {
            services: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Connect to all MCP servers from the app configuration.
    pub(crate) async fn connect(&self, config: &AppConfig) -> Result<(), String> {
        let Some(ref servers) = config.mcp_servers else {
            return Ok(());
        };

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

    async fn connect_stdio(
        &self,
        name: &str,
        config: &crate::infra::config::types::McpServerConfig,
    ) -> Result<(), String> {
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

    async fn connect_sse(
        &self,
        name: &str,
        config: &crate::infra::config::types::McpServerConfig,
    ) -> Result<(), String> {
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
    pub(crate) async fn list_all_tools(&self) -> Vec<(String, String, String, Value)> {
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
    pub(crate) async fn call_tool(
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
    pub(crate) async fn shutdown(&self) {
        let mut services = self.services.lock().await;
        for (name, mut service) in services.drain() {
            if let Err(e) = service.close().await {
                tracing::warn!(server = %name, error = %e, "MCP server shutdown error");
            }
        }
    }
}

/// Adapter wrapping an MCP tool as an `adk_core::Tool`.
///
/// The publicly-facing name follows the convention `mcp:{server_id}:{name}`
/// to avoid naming conflicts with built-in tools.
pub(crate) struct McpToolAdapter {
    full_name: String,
    description: String,
    parameters_schema: Option<Value>,
    manager: Arc<McpClientManager>,
}

impl McpToolAdapter {
    pub(crate) fn new(
        server_id: String,
        tool_name: String,
        description: String,
        parameters_schema: Option<Value>,
        manager: Arc<McpClientManager>,
    ) -> Self {
        let full_name = format!("mcp:{server_id}:{tool_name}");
        Self {
            full_name,
            description,
            parameters_schema,
            manager,
        }
    }
}

#[async_trait]
impl adk_core::Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Option<Value> {
        self.parameters_schema.clone()
    }

    async fn execute(
        &self,
        _ctx: Arc<dyn adk_core::ToolContext>,
        args: Value,
    ) -> adk_core::Result<Value> {
        // Parse the server_id from the prefixed name.
        // Format: mcp:{server_id}:{tool_name}
        let parts: Vec<&str> = self.full_name.splitn(3, ':').collect();
        let server_id = parts.get(1).unwrap_or(&"unknown");
        let tool_name = parts.get(2).unwrap_or(&"unknown");

        self.manager
            .call_tool(server_id, tool_name, args)
            .await
            .map_err(|e| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::Internal,
                    "mcp_tool_error",
                    format!("MCP call to {} failed: {}", self.full_name, e),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adk_core::Tool;

    #[test]
    fn test_mcp_tool_adapter_name_format() {
        let manager = Arc::new(McpClientManager::new());
        let adapter = McpToolAdapter::new(
            "filesystem".into(),
            "read_file".into(),
            "Read a file".into(),
            None,
            manager,
        );
        assert_eq!(adapter.name(), "mcp:filesystem:read_file");
        assert_eq!(adapter.description(), "Read a file");
        assert!(adapter.parameters_schema().is_none());
    }

    #[test]
    fn test_mcp_tool_adapter_name_with_schema() {
        let manager = Arc::new(McpClientManager::new());
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            }
        });
        let adapter = McpToolAdapter::new(
            "git".into(),
            "status".into(),
            "Git status".into(),
            Some(schema.clone()),
            manager,
        );
        assert_eq!(adapter.name(), "mcp:git:status");
        assert_eq!(adapter.parameters_schema(), Some(schema));
    }

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
}
