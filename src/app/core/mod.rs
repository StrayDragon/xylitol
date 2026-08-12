//! Cross-surface seams shared by all application entry points.
//!
//! Everything under `app/core/` is the **shared seam layer**: privileged
//! modules that more than one surface depends on. They are distinct from the
//! self-contained surface directories (`cli/`, `server/`, `tui/`,
//! `gui.rs`) and are the only `app/` modules allowed to import `agent` and/or
//! `infra`.
//!
//! - `bootstrap` — shared assembly path from CLI args to a constructed
//!   agent (config → registry → trust → resources → build_agent). print /
//!   tui / server call `bootstrap`; ingredient-only paths (e.g. --list-models) call
//!   `resolve_assembly` (they need ingredients without building).
//! - [`composition`] — the composition root: the single module permitted
//!   to import both `agent` and `infra`, centralizing Agent wiring so CLI/RPC/
//!   Server/TUI never duplicate it.
//! - [`mcp_spec`] — embed-facing MCP server description (`McpServerSpec`).
//! - [`dispatch`] — shared Command execution: maps non-transport
//!   `protocol::Command` variants to [`driver::XyDriver`] method calls.
//!   Consumed by tui (spec ce10).
//! - [`driver`] — the runtime boundary (`XyDriver` trait + `XyInProcessDriver`/
//!   `XyRemoteDriver`). Surfaces depend on this, never on `agent` internals.
//!   `XyInProcessDriver` MAY call documented surface infra (trust / clipboard /
//!   config read); full agent+tool assembly stays in `composition`.

pub mod bang_exec;
pub(crate) mod bootstrap;
pub(crate) mod composition;
pub(crate) mod dispatch;
pub(crate) mod driver;
pub(crate) mod driver_error;
pub(crate) mod lag;
pub(crate) mod mcp_spec;
pub mod product_commands;
pub mod session_export;
/// Session-adjacent tool spill root (c2080 append-only / hard-truncation).
pub mod tool_spill;
