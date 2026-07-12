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
//! // Do not name `AgentRuntime` at the call site.
//! let _ = input.caller;
//! ```
//!
//! ## Known leaks (not stability promises)
//!
//! - [`BootstrappedAgent::agent`] still exposes `AgentRuntime` — use
//!   [`BootstrappedAgent::into_runtime`] / [`BootstrappedAgent::into_driver`].
//!
//! ## Not exported here
//!
//! - `dispatch` / `RemoteDriver` — stay crate-internal until a surface wires
//!   them. Reach via `app::core` only inside this crate.
//!
//! See `docs/architecture/库与多客户端.md`.

pub use crate::app::core::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, BootstrappedAgent, BootstrappedRuntime,
    bootstrap,
};
pub use crate::app::core::composition::{
    BuildAgentOptions, McpServerSpec, McpSession, McpTransportSpec, build_agent,
};
pub use crate::app::core::driver::{
    CommandInfo, Driver, EventStream, InProcessDriver, ModelInfo, SessionState, SessionStats,
};
