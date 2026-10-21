use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct McpServerConfig {
    pub name: String,
    #[serde(default = "default_mcp_transport")]
    pub transport: McpTransportKind,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
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
