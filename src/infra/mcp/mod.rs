//! MCP (Model Context Protocol) client integration.
//!
//! Provides [`McpClientManager`] to connect to MCP servers via stdio or SSE
//! transport, and [`McpToolAdapter`] to expose MCP tools as [`crate::runtime_protocol::XyTool`].
//!
//! This module is independent of the skills system and can be used standalone.

mod adapter;
mod assemble;
mod client;
pub(crate) mod types;

pub use adapter::McpToolAdapter;
pub use assemble::{
    adapters_from_discovered, connect_and_discover, connect_and_discover_from_config, mcp_enabled,
};
pub use client::McpClientManager;
pub use types::{McpServerConfig, McpTransportKind};
