//! # xylitol
//!
//! LLM-Augmented Development Toolkit.
//!
//! ## Public API = `embed` + curated `Xy*` re-exports
//!
//! The only externally reachable surface is the [`embed`] seam plus the
//! curated re-exports below. Composition lives in [`wiring`]. Occupancy
//! (session slots) stays in [`wiring`] until `XyInProcessDriver` moves to
//! `xylitol-runtime`.
//!
//! Ports: [`XyModel`], [`XyTool`], [`XySessionStore`], [`XyEventSink`],
//! [`XyPermission`], [`XyBashExecutor`], [`XyExportIo`], [`XySecretResolver`],
//! [`XyModelBuilder`], [`XyHookBus`].
//! Shared application protocol: [`XyDriver`], [`XyInProcessDriver`], [`XyDriverError`].
//! Envelope client: [`HostClient`], [`InProcessClient`].
//! Events / stream: [`XyEvent`], [`XyChunk`], [`XyStream`].
//! Hook outcomes: [`XyHookOutcome`], [`NoopHookBus`].
//! Errors: [`XyError`], [`XyToolError`], [`XySessionStoreError`], [`XySessionError`], [`XyExportError`], [`XyTrustError`], [`XyDriverError`].
//! Config metadata: [`XyModelConfig`], [`XyModelKind`], [`XyModelMeta`],
//! [`XyToolSchema`].
//!
//! ## Embed seam ([`embed`])
//!
//! Multi-client / external-crate assembly: [`embed::bootstrap`],
//! [`embed::BootstrappedRuntime`] / [`embed::BootstrappedAgent::into_runtime`],
//! [`embed::XyInProcessDriver`], [`embed::XyDriver`], [`embed::BuildAgentOptions`],
//! [`embed::McpSession`]. Script hooks are configured via
//! [`embed::BuildAgentOptions::hooks_config`]; replaceable port is [`XyHookBus`].
//!
//! Visibility is enforced by the compiler, not convention:
//!
//! ```compile_fail
//! // Internal helpers are not a public crate root module.
//! use xylitol::utils::sync::lock_mutex;
//! ```
//!
//! ```rust
//! // The sanctioned seam compiles.
//! use xylitol::embed::BootstrapInput;
//! use xylitol::XyDriver as _;
//! ```

pub mod embed;
pub mod wiring;

#[cfg(feature = "cli")]
pub mod cli;

// ── Curated `pub use` ───────────────────────────────────────────────

#[cfg(feature = "server")]
pub use crate::wiring::attach::attach_preflight_with;
pub use crate::wiring::attach::{
    DEFAULT_ATTACH_URL, attach_fail_message, probe_host, resolve_attach_url,
};
#[cfg(feature = "server")]
pub use crate::wiring::driver::{LinkHealth, LinkTunings, XyRemoteDriver};
pub use crate::wiring::driver::{XyDriver, XyDriverError, XyInProcessDriver};
#[cfg(feature = "server")]
pub use crate::wiring::host_client::HttpWsClient;
pub use crate::wiring::host_client::{HostClient, HostClientError, InProcessClient, MuxStream};
pub use crate::wiring::session_export::SessionExporter;
pub use xylitol_plugin_sdk::error::{
    XyError, XyExportError, XySessionError, XySessionStoreError, XyToolError, XyTrustError,
};
pub use xylitol_plugin_sdk::lifecycle::{XyEvent, XyEventError};
pub use xylitol_plugin_sdk::model::{
    XyChunk, XyModelConfig, XyModelKind, XyModelMeta, XyToolSchema,
};
pub use xylitol_plugin_sdk::{
    NoopHookBus, XyBashExecutor, XyEventSink, XyExportIo, XyHookBus, XyHookOutcome, XyModel,
    XyModelBuilder, XyPermission, XySecretResolver, XySessionStore, XyStream, XyTool,
};

/// Application entry point.
#[cfg(feature = "cli")]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    cli::run().await
}

#[cfg(test)]
#[path = "../tests/inner/provider_http_stream_abort.rs"]
mod provider_http_stream_abort;

#[cfg(test)]
#[path = "../tests/inner/token_estimator_bench.rs"]
mod token_estimator_bench;

#[cfg(test)]
#[path = "../tests/inner/lab_compaction_replay.rs"]
mod lab_compaction_replay;

#[cfg(test)]
#[path = "../tests/inner/session_fold.rs"]
mod session_fold;
