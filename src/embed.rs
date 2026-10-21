//! Documented embed seam for external crates and custom clients.
//!
//! Prefer this module plus the curated `Xy*` re-exports in the crate root over
//! reaching into `agent::capabilities` or wiring concrete types.
//!
//! Typical path: [`boot`] / [`BootOptions`] / [`Profile`], or the synchronous
//! [`bootstrap`] seam.
//!
//! ```rust
//! use xylitol::embed::{BootOptions, Profile};
//!
//! let opts = BootOptions {
//!     profile: Profile::Embed,
//!     config_path: None,
//!     session: None,
//!     model: None,
//!     trust_override: Some(false),
//!     interactive: false,
//!     caller: "embed-example",
//!     ephemeral_sessions: false,
//! };
//! // Then: `boot(opts).await?.runtime.driver` → `XyDriver::run`.
//! // Do not name `AgentRuntime` at the call site.
//! let _ = opts.caller;
//! ```
//!
//! ## Script hooks (library port)
//!
//! Configure via [`BuildAgentOptions::hooks_config`] (empty = zero-cost).
//! The replaceable port is crate-root [`crate::XyHookBus`] / [`crate::XyHookOutcome`]
//! (and [`crate::NoopHookBus`]). Prefer those over `xylitol_official_plugin_hooks` types.
//!
//! ## Not exported here
//!
//! - `dispatch` / `XyRemoteDriver` — stay crate-internal until a surface wires
//!   them. Reach via crate modules only inside this crate.
//! - `HookDispatcher` / `HookEvent` — script-hook implementation details.
//!
//! See `llmanspec/specs/protocol-app/` and root `AGENTS.md` (client / host roles).

pub use xylitol_kernel::boot::{BootOptions, Kernel, Profile, boot};
pub use xylitol_kernel::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, BootstrappedAgent, BootstrappedRuntime,
    bootstrap,
};
pub use xylitol_kernel::composition::{
    BuildAgentOptions, McpServerSpec, McpSession, McpTransportSpec, build_agent,
};
pub use xylitol_kernel::driver::{
    CommandInfo, EventStream, ModelInfo, SessionState, SessionStats, XyComposedDriver, XyDriver,
    XyDriverError,
};
