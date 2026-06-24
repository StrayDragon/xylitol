//! MCP (Model Context Protocol) client integration.
//!
//! Provides [`McpClientManager`] to connect to MCP servers via stdio or SSE
//! transport, and [`McpToolAdapter`] to expose MCP tools as [`XyTool`].
//!
//! This module is independent of the skills system and can be used standalone.

mod adapter;
mod client;
pub(crate) mod types;

pub use adapter::McpToolAdapter;
pub use client::McpClientManager;
pub use types::{McpServerConfig, McpTransportKind};
