//! MCP (Model Context Protocol) client integration.
//!
//! Provides [`McpClientManager`] to connect to MCP servers via stdio or SSE
//! transport, and [`McpToolAdapter`] to expose MCP tools as [`crate::protocol::ports::XyTool`].
//!
//! This module is independent of the skills system and can be used standalone.

mod adapter;
mod assemble;
mod client;
mod defaults;
pub(crate) mod types;

#[cfg(test)]
pub use adapter::McpToolAdapter;
pub use assemble::{connect_and_discover, connect_and_discover_with_progress, mcp_enabled};
pub use client::{
    ConnectedMcpServer, McpClientManager, McpConnectDiagnostic, McpConnectProgress, McpError,
};
pub use defaults::MCP_SERVER_CONNECT_TIMEOUT;
#[cfg(test)]
pub use types::McpTransportKind;
