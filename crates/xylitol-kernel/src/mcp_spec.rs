use std::collections::HashMap;

use xylitol_official_plugin_config::types::{McpServerConfig, McpTransportKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpTransportSpec {
    Stdio,
    Sse,
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
}

impl From<&McpServerConfig> for McpServerSpec {
    fn from(c: &McpServerConfig) -> Self {
        Self {
            name: c.name.clone(),
            transport: match c.transport {
                McpTransportKind::Stdio => McpTransportSpec::Stdio,
                McpTransportKind::Sse => McpTransportSpec::Sse,
            },
            command: c.command.clone(),
            args: c.args.clone(),
            url: c.url.clone(),
            env: c.env.clone(),
            headers: c.headers.clone(),
        }
    }
}

impl From<&McpServerSpec> for McpServerConfig {
    fn from(s: &McpServerSpec) -> Self {
        Self {
            name: s.name.clone(),
            transport: match s.transport {
                McpTransportSpec::Stdio => McpTransportKind::Stdio,
                McpTransportSpec::Sse => McpTransportKind::Sse,
            },
            command: s.command.clone(),
            args: s.args.clone(),
            url: s.url.clone(),
            env: s.env.clone(),
            headers: s.headers.clone(),
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
