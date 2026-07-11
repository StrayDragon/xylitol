//! Documented embed seam for external crates and custom clients.
//!
//! Prefer this module plus the curated `Xy*` re-exports in the crate root over
//! reaching into `agent::session` or `infra` concrete types.
//!
//! Typical path:
//!
//! ```rust
//! use xylitol::embed::BootstrapInput;
//!
//! let input = BootstrapInput {
//!     config_path: None,
//!     session: None,
//!     model: None,
//!     trust_override: Some(false),
//!     interactive: false,
//!     caller: "embed-example",
//! };
//! // Then: `bootstrap(input)?.into_runtime()` → `Driver::run`.
//! // Do not name `ReActAgent` at the call site.
//! let _ = input.caller;
//! ```
//!
//! ## Known leaks (not stability promises)
//!
//! - [`BootstrappedAgent::agent`] still exposes `ReActAgent` — use
//!   [`BootstrappedAgent::into_runtime`] / [`BootstrappedAgent::into_driver`].
//! - `mcp_servers` / [`McpSession::reload`] still use infra
//!   `McpServerConfig` until a seam type exists.
//!
//! ## Not exported here
//!
//! - `dispatch` / `RemoteDriver` — stay crate-internal until a surface wires
//!   them (Server/TUI / c540). Reach via `app::core` only inside this crate.
//!
//! See `docs/architecture/library-and-clients.md`.

pub use crate::app::core::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, BootstrappedAgent, BootstrappedRuntime,
    bootstrap,
};
pub use crate::app::core::composition::{BuildAgentOptions, McpSession, build_agent};
pub use crate::app::core::driver::{
    CommandInfo, Driver, EventStream, InProcessDriver, ModelInfo, SessionState, SessionStats,
};
