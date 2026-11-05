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
    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    #[serde(default = "default_call_timeout_ms")]
    pub call_timeout_ms: u64,
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
            connect_timeout_ms: default_connect_timeout_ms(),
            call_timeout_ms: default_call_timeout_ms(),
        }
    }
}

fn default_connect_timeout_ms() -> u64 {
    8_000
}

fn default_call_timeout_ms() -> u64 {
    120_000
}

impl McpServerConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("mcp server name must not be empty".into());
        }
        if self.connect_timeout_ms == 0 {
            return Err("connect_timeout_ms must be positive".into());
        }
        if self.call_timeout_ms == 0 {
            return Err("call_timeout_ms must be positive".into());
        }
        match self.transport {
            McpTransportKind::Stdio => match self.command.as_deref().map(str::trim) {
                None | Some("") => Err("command is required for stdio transport".into()),
                Some(_) => Ok(()),
            },
            McpTransportKind::Http => match self.url.as_deref().map(str::trim) {
                None | Some("") => Err("url is required for http transport".into()),
                Some(u) if u.ends_with("/sse") => Err(format!(
                    "legacy HTTP+SSE endpoint is not supported: {u}; use Streamable HTTP"
                )),
                Some(u) if !(u.starts_with("http://") || u.starts_with("https://")) => {
                    Err(format!("url must be http(s) for http transport: {u}"))
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

#[derive(Clone, Copy, Debug, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKind {
    Stdio,
    Http,
}

impl<'de> Deserialize<'de> for McpTransportKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        match raw.as_str() {
            "stdio" => Ok(Self::Stdio),
            "http" => Ok(Self::Http),
            "sse" => Err(serde::de::Error::custom(
                "`transport: sse` is removed; use `http` (Streamable HTTP). Legacy HTTP+SSE is not supported.",
            )),
            other => Err(serde::de::Error::custom(format!(
                "unknown MCP transport `{other}`; expected `stdio` or `http`"
            ))),
        }
    }
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
    fn validate_http_requires_url() {
        let cfg = McpServerConfig {
            name: "x".into(),
            transport: McpTransportKind::Http,
            ..Default::default()
        };
        assert!(cfg.validate().unwrap_err().contains("url"));
    }

    #[test]
    fn sse_transport_is_rejected_with_migration_hint() {
        let err = serde_json::from_value::<McpServerConfig>(serde_json::json!({
            "name": "x",
            "transport": "sse"
        }))
        .expect_err("sse must be rejected");
        let message = err.to_string();
        assert!(message.contains("sse"), "{message}");
        assert!(message.contains("http"), "{message}");
    }

    #[test]
    fn legacy_sse_url_is_rejected() {
        let cfg = McpServerConfig {
            name: "x".into(),
            transport: McpTransportKind::Http,
            url: Some("https://example.com/sse".into()),
            ..Default::default()
        };
        let message = cfg.validate().unwrap_err();
        assert!(message.contains("legacy HTTP+SSE"), "{message}");
    }

    #[test]
    fn timeout_defaults_and_validation() {
        let cfg = McpServerConfig::default();
        assert_eq!(cfg.connect_timeout_ms, 8_000);
        assert_eq!(cfg.call_timeout_ms, 120_000);
        let zero = McpServerConfig {
            name: "x".into(),
            command: Some("bin".into()),
            call_timeout_ms: 0,
            ..Default::default()
        };
        assert!(zero.validate().unwrap_err().contains("call_timeout_ms"));
    }
}
