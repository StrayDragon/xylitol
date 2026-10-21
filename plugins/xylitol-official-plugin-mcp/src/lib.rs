mod adapter;
mod assemble;
mod client;
mod defaults;
mod hub;
pub mod plugin;
pub(crate) mod types;

pub use adapter::McpToolAdapter;
pub use assemble::{connect_and_discover, connect_and_discover_with_progress, mcp_enabled};
pub use client::{
    ConnectedMcpServer, McpClientManager, McpConnectDiagnostic, McpConnectProgress, McpError,
};
pub use config::{McpServerConfig, McpTransportKind};
pub use defaults::MCP_SERVER_CONNECT_TIMEOUT;
pub use hub::McpPrepareHub;

pub mod config;
