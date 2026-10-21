use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::client::McpClientManager;
use xylitol_plugin_sdk::tool_name::{is_provider_safe_tool_name, mcp_tool_public_name};

pub struct McpToolAdapter {
    full_name: String,
    server_id: String,
    tool_name: String,
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
        let full_name = mcp_tool_public_name(&server_id, &tool_name);
        debug_assert!(
            is_provider_safe_tool_name(&full_name),
            "MCP tool name must be provider-safe: {full_name}"
        );
        Self {
            full_name,
            server_id,
            tool_name,
            description,
            parameters_schema,
            manager,
        }
    }
}

#[async_trait]
impl xylitol_plugin_sdk::XyTool for McpToolAdapter {
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
        _ctx: &xylitol_plugin_sdk::XyToolCtx,
        args: Value,
    ) -> Result<String, xylitol_plugin_sdk::error::XyToolError> {
        let result = self
            .manager
            .call_tool(&self.server_id, &self.tool_name, args)
            .await
            .map_err(|e| {
                xylitol_plugin_sdk::error::XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "MCP call to {} failed: {}",
                    self.full_name,
                    e
                ))
            })?;

        serde_json::to_string(&result).map_err(|e| {
            xylitol_plugin_sdk::error::XyToolError::ExecutionFailed(anyhow::anyhow!(
                "failed to serialize MCP result: {}",
                e
            ))
        })
    }

    fn execution_mode(&self) -> xylitol_plugin_sdk::XyToolExecutionMode {
        xylitol_plugin_sdk::XyToolExecutionMode::Sequential
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::XyTool;

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
        assert_eq!(adapter.name(), "mcp__filesystem__read_file");
        assert!(xylitol_plugin_sdk::is_provider_safe_tool_name(
            adapter.name()
        ));
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
        assert_eq!(adapter.name(), "mcp__git__status");
        assert_eq!(adapter.parameters_schema(), schema);
    }

    #[test]
    fn test_mcp_tool_adapter_execution_mode_is_barrier() {
        let manager = Arc::new(McpClientManager::new());
        let adapter = McpToolAdapter::new(
            "filesystem".into(),
            "read_file".into(),
            "Read a file".into(),
            None,
            manager,
        );
        assert_eq!(
            adapter.execution_mode(),
            xylitol_plugin_sdk::XyToolExecutionMode::Sequential
        );
    }
}
