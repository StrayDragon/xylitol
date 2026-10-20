//! Composition root: apply official Host plugins and inject runtime.
//!
//! Empty MCP / hooks skip `apply`. Occupancy policy lives in `xylitol-serve`;
//! this module only assembles the factory and bind.

pub(crate) mod assembly;
pub mod bootstrap;
pub mod composition;
pub(crate) mod mcp_spec;

pub mod attach;
pub mod dispatch;
pub mod driver;
pub mod infra;
pub(crate) mod lag;
pub mod product_commands;
pub(crate) mod tool_spill;

#[cfg(feature = "server")]
pub mod occupancy;
