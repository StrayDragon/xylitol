use std::collections::HashMap;

use xylitol_official_plugin_config::types::{
    McpOAuthConfig, McpScope, McpServerConfig, McpTransportKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpTransportSpec {
    Stdio,
    Http,
    Custom,
}

#[derive(Clone, Debug)]
pub struct McpServerSpec {
    pub name: String,
    pub transport: McpTransportSpec,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub headers: Option<HashMap<String, String>>,
    pub scope: McpScope,
    pub credential: Option<String>,
    pub connect_timeout_ms: u64,
    pub call_timeout_ms: u64,
    pub custom_transport: Option<String>,
    pub oauth: Option<McpOAuthConfig>,
}

impl From<&McpServerConfig> for McpServerSpec {
    fn from(c: &McpServerConfig) -> Self {
        Self {
            name: c.name.clone(),
            transport: match c.transport {
                McpTransportKind::Stdio => McpTransportSpec::Stdio,
                McpTransportKind::Http => McpTransportSpec::Http,
                McpTransportKind::Custom => McpTransportSpec::Custom,
            },
            command: c.command.clone(),
            args: c.args.clone(),
            url: c.url.clone(),
            env: c.env.clone(),
            headers: c.headers.clone(),
            scope: c.scope,
            credential: c.credential.clone(),
            connect_timeout_ms: c.connect_timeout_ms,
            call_timeout_ms: c.call_timeout_ms,
            custom_transport: c.custom_transport.clone(),
            oauth: c.oauth.clone(),
        }
    }
}

impl From<&McpServerSpec> for McpServerConfig {
    fn from(s: &McpServerSpec) -> Self {
        Self {
            name: s.name.clone(),
            transport: match s.transport {
                McpTransportSpec::Stdio => McpTransportKind::Stdio,
                McpTransportSpec::Http => McpTransportKind::Http,
                McpTransportSpec::Custom => McpTransportKind::Custom,
            },
            command: s.command.clone(),
            args: s.args.clone(),
            url: s.url.clone(),
            env: s.env.clone(),
            headers: s.headers.clone(),
            scope: s.scope,
            credential: s.credential.clone(),
            connect_timeout_ms: s.connect_timeout_ms,
            call_timeout_ms: s.call_timeout_ms,
            custom_transport: s.custom_transport.clone(),
            oauth: s.oauth.clone(),
        }
    }
}

impl McpServerSpec {
    pub fn from_infra_list(servers: Option<Vec<McpServerConfig>>) -> Option<Vec<Self>> {
        servers.map(|v| v.iter().map(Self::from).collect())
    }

    pub fn to_infra_list(servers: &[Self]) -> Vec<McpServerConfig> {
        servers.iter().map(McpServerConfig::from).collect()
    }
}
