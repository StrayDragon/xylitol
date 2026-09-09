//! Documented embed seam for external crates and custom clients.
//!
//! Prefer this module plus the curated `Xy*` re-exports in the crate root over
//! reaching into `agent::capabilities` or `infra` concrete types.
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
//! // Then: `bootstrap(input)?.into_runtime()` → `XyDriver::run`.
//! // Do not name `AgentRuntime` at the call site.
//! let _ = input.caller;
//! ```
//!
//! ## Script hooks (library port)
//!
//! Configure via [`BuildAgentOptions::hooks_config`] (empty = zero-cost).
//! The replaceable port is crate-root [`crate::XyHookBus`] / [`crate::XyHookOutcome`]
//! (and [`crate::NoopHookBus`]). Prefer those over `infra::hooks` types.
//!
//! ## Not exported here
//!
//! - `dispatch` / `XyRemoteDriver` — stay crate-internal until a surface wires
//!   them. Reach via `app::core` only inside this crate.
//! - `HookDispatcher` / `HookEvent` — infra script implementation details.
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
    CommandInfo, EventStream, ModelInfo, SessionState, SessionStats, XyDriver, XyDriverError,
    XyInProcessDriver,
};
