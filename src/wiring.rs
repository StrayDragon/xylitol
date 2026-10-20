//! Composition root: apply official Host plugins and inject runtime.
//!
//! Empty MCP / hooks skip `apply`. Occupancy and Driver stay here until
//! `XyInProcessDriver` lives in `xylitol-runtime` (runtime ↛ envelope / plugins).

pub(crate) mod assembly;
pub mod bootstrap;
pub mod composition;
pub(crate) mod mcp_spec;

pub mod attach;
pub mod bang_exec;
pub mod dispatch;
pub mod driver;
pub(crate) mod driver_error;
pub mod host_client;
pub mod infra;
pub(crate) mod lag;
pub mod product_commands;
pub mod session_export;
pub(crate) mod tool_spill;
pub(crate) mod utils;

#[cfg(feature = "server")]
pub mod occupancy;
