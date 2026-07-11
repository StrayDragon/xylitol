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
//! // Then: `bootstrap(input)?` → `InProcessDriver::new(agent, store)` → `Driver::run`.
//! let _ = input.caller;
//! ```
//!
//! See `docs/architecture/library-and-clients.md`.

pub use crate::app::core::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, BootstrappedAgent, bootstrap,
};
pub use crate::app::core::composition::{BuildAgentOptions, McpSession, build_agent};
pub use crate::app::core::dispatch::{
    DispatchError, DispatchOutcome, dispatch, parse_thinking_level,
};
pub use crate::app::core::driver::{
    CommandInfo, Driver, EventStream, InProcessDriver, ModelInfo, SessionState, SessionStats,
};
