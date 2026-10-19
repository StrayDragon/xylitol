//! MCP server configuration vocabulary (loaded by composition-root YAML).

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct McpServerConfig {
    pub name: String,
    /// Transport kind: "stdio" or "sse".
    #[serde(default = "default_mcp_transport")]
    pub transport: McpTransportKind,
    /// Command to spawn (stdio transport only).
    #[serde(default)]
    pub command: Option<String>,
    /// Arguments for the command (stdio transport only).
    #[serde(default)]
    pub args: Option<Vec<String>>,
    /// SSE / streamable-HTTP endpoint URL (sse transport only).
    #[serde(default)]
    pub url: Option<String>,
    /// Extra environment variables for the child process (stdio transport only).
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    /// Extra HTTP headers for SSE / streamable-HTTP (e.g. API keys).
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            transport: default_mcp_transport(),
            command: None,
            args: None,
            url: None,
            env: None,
            headers: None,
        }
    }
}

impl McpServerConfig {
    /// Validate required fields for the selected transport (c1080 / mcp4).
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("mcp server name must not be empty".into());
        }
        match self.transport {
            McpTransportKind::Stdio => match self.command.as_deref().map(str::trim) {
                None | Some("") => Err("command is required for stdio transport".into()),
                Some(_) => Ok(()),
            },
            McpTransportKind::Sse => match self.url.as_deref().map(str::trim) {
                None | Some("") => Err("url is required for sse transport".into()),
                Some(u) if !(u.starts_with("http://") || u.starts_with("https://")) => {
                    Err(format!("url must be http(s) for sse transport: {u}"))
                }
                Some(_) => Ok(()),
            },
        }
    }
}

fn default_mcp_transport() -> McpTransportKind {
    McpTransportKind::Stdio
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKind {
    Stdio,
    Sse,
}
