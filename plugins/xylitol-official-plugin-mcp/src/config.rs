use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    #[serde(default)]
    pub scope: McpScope,
    #[serde(default)]
    pub credential: Option<String>,
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
            scope: McpScope::default(),
            credential: None,
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

    pub fn fingerprint(&self) -> String {
        let identity = serde_json::json!({
            "name": self.name,
            "transport": self.transport,
            "command": self.command,
            "args": self.args,
            "url": self.url,
            "env": self.env,
            "headers": self.headers,
        });
        let bytes = serde_json::to_vec(&identity).unwrap_or_default();
        let digest = Sha256::digest(&bytes);
        digest.iter().map(|b| format!("{b:02x}")).collect()
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

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum McpScope {
    #[default]
    Workspace,
    Shared,
    Session,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(name: &str, command: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.into(),
            command: Some(command.into()),
            ..Default::default()
        }
    }

    #[test]
    fn fingerprint_ignores_scope_and_credential() {
        let mut left = server("fs", "fs-server");
        let right = McpServerConfig {
            scope: McpScope::Shared,
            credential: Some("a".into()),
            ..left.clone()
        };
        assert_eq!(left.fingerprint(), right.fingerprint());
        left.args = Some(vec!["--x".into()]);
        assert_ne!(left.fingerprint(), right.fingerprint());
    }

    #[test]
    fn validate_sse_requires_url() {
        let cfg = McpServerConfig {
            name: "x".into(),
            transport: McpTransportKind::Sse,
            ..Default::default()
        };
        assert!(cfg.validate().unwrap_err().contains("url"));
    }
}
