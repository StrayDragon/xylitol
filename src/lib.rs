//! # xylitol
//!
//! LLM-Augmented Development Toolkit.
//!
//! ## Curated public API (`Xy*` contracts)
//!
//! Stable library entry points are re-exported below. Prefer these over
//! reaching into `domain` / `runtime_protocol` modules by path. Everything else
//! under `pub mod` is crate structure for the binary / advanced embedding —
//! not a stability promise.
//!
//! Ports: [`XyModel`], [`XyTool`], [`XySessionStore`], [`XyEventSink`],
//! [`XyPermission`], [`XyBashExecutor`], [`XyExportIo`], [`XySecretResolver`],
//! [`XyModelBuilder`], [`XyHookBus`].
//! Events / stream: [`XyEvent`], [`XyChunk`], [`XyStream`].
//! Hook outcomes: [`XyHookOutcome`], [`NoopHookBus`].
//! Errors: [`XyError`], [`XyToolError`].
//! Config metadata: [`XyModelConfig`], [`XyModelKind`], [`XyModelMeta`],
//! [`XyToolSchema`].
//!
//! ## Embed seam ([`embed`])
//!
//! Multi-client / external-crate assembly: [`embed::bootstrap`],
//! [`embed::BootstrappedRuntime`] / [`embed::BootstrappedAgent::into_runtime`],
//! [`embed::InProcessDriver`], [`embed::Driver`], [`embed::BuildAgentOptions`],
//! [`embed::McpSession`]. Script hooks are configured via
//! [`embed::BuildAgentOptions::hooks_config`]; replaceable port is [`XyHookBus`].
//! Do **not** treat `infra::*` or `agent::session::*` as a stability promise —
//! extend the seam instead.
//!
//! Not exported from `embed`: `dispatch`, `RemoteDriver`, infra concrete types
//! (`HookDispatcher`, `HookEvent`).

pub mod agent;
pub mod app;
pub mod domain;
pub mod embed;
pub mod infra;
pub mod protocol;
pub mod runtime_protocol;

// ── Curated `pub use` (c500 / architecture.ar09) ─────────────────────

pub use crate::domain::error::{XyError, XyToolError};
pub use crate::domain::lifecycle::XyEvent;
pub use crate::domain::model::{XyModelConfig, XyModelKind};
pub use crate::domain::types::{XyChunk, XyModelMeta, XyToolSchema};
pub use crate::runtime_protocol::{
    NoopHookBus, XyBashExecutor, XyEventSink, XyExportIo, XyHookBus, XyHookOutcome, XyModel,
    XyModelBuilder, XyPermission, XySecretResolver, XySessionStore, XyStream, XyTool,
};

/// Application entry point.
#[cfg(feature = "cli")]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    app::cli::run().await
}

#[cfg(test)]
mod tests;
