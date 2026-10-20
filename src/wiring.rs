//! Composition root: inject official Host plugins into runtime.
//!
//! Empty MCP / hooks skip start. Occupancy policy lives in `xylitol-serve`;
//! this module only assembles the factory and bind.

pub(crate) mod assembly;
pub mod bootstrap;
pub mod composition;
pub(crate) mod mcp_spec;

pub mod attach;
pub mod config;
pub mod dispatch;
pub mod driver;
pub mod event;
pub mod export;
pub(crate) mod lag;
pub mod observability;
pub mod permission;
pub mod product_commands;
pub mod provider;
pub mod settings;
pub mod timing;
pub(crate) mod tool_spill;

#[cfg(feature = "server")]
pub mod occupancy;
