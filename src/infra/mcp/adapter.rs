//! MCP tool adapter — wraps MCP tools as [`crate::protocol::ports::XyTool`].

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::client::McpClientManager;

/// Adapter wrapping an MCP tool as an [`crate::protocol::ports::XyTool`].
///
/// The publicly-facing name follows the convention `mcp:{server_id}:{name}`
/// to avoid naming conflicts with built-in tools.
pub struct McpToolAdapter {
    full_name: String,
    description: String,
    parameters_schema: Option<Value>,
    manager: Arc<McpClientManager>,
}

impl McpToolAdapter {
    pub fn new(
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
impl crate::protocol::ports::XyTool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.parameters_schema
            .clone()
            .unwrap_or(serde_json::json!({}))
    }

    async fn execute(
        &self,
        _ctx: &crate::protocol::ports::XyToolCtx,
        args: Value,
    ) -> Result<String, crate::protocol::error::XyToolError> {
        let parts: Vec<&str> = self.full_name.splitn(3, ':').collect();
        let server_id = parts.get(1).unwrap_or(&"unknown");
        let tool_name = parts.get(2).unwrap_or(&"unknown");

        let result = self
            .manager
            .call_tool(server_id, tool_name, args)
            .await
            .map_err(|e| {
                crate::protocol::error::XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "MCP call to {} failed: {}",
                    self.full_name,
                    e
                ))
            })?;

        serde_json::to_string(&result).map_err(|e| {
            crate::protocol::error::XyToolError::ExecutionFailed(anyhow::anyhow!(
                "failed to serialize MCP result: {}",
                e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ports::XyTool;

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
        assert_eq!(adapter.parameters_schema(), serde_json::json!({}));
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
        assert_eq!(adapter.parameters_schema(), schema);
    }
}
