//! # xylitol
//!
//! LLM-Augmented Development Toolkit.
//!
//! ## Public API = `embed` + curated `Xy*` re-exports
//!
//! The only externally reachable surface is the [`embed`] seam plus the
//! curated re-exports below. `agent` / `infra` are public since c2837 (BDD
//! independent integration-test target needs them reachable outside the
//! crate); `utils` stays `pub(crate)`; `protocol::wire` / `protocol::ports`
//! and the root `Xy*` re-exports are the shared vocabulary. There is no
//! second stable face: deep module paths are not a stability promise —
//! extend the seam instead.

// c2837: BDD 独立集成测试 target 使 agent/infra 公开化，rustdoc 随之扫描扩大
// 后的 doc 面；大量 intra-doc 链接指向私有项/断链在迁移前不可见（未开
// --document-private-items）。这是 doc 质量软提示而非硬错误——允许跳过，
// 完整 doc 链接治理另立 change。
#![allow(rustdoc::broken_intra_doc_links)]
#![allow(rustdoc::private_intra_doc_links)]
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
//! Not exported from `embed`: `dispatch`, `XyRemoteDriver`, infra concrete types
//! (`HookDispatcher`, `HookEvent`).
//!
//! Visibility is enforced by the compiler, not convention:
//!
//! Since c2837, `agent` / `infra` are public (the BDD independent
//! integration-test target reaches them as `xylitol::agent::…`); `utils`
//! remains `pub(crate)` and is still a hard boundary:
//!
//! ```compile_fail
//! // `utils` is `pub(crate)` — external crates cannot reach it.
//! use xylitol::utils::sync::lock_mutex;
//! ```
//!
//! ```rust
//! // The sanctioned seam compiles.
//! use xylitol::embed::BootstrapInput;
//! use xylitol::XyDriver as _;
//! ```

// `agent`/`infra` 由 pub(crate) 升 pub：BDD 独立集成测试 target（tests/bdd.rs）
// 只能访问 lib 的 pub 项；纯可见性扩张，无行为变化（c2837 编译隔离）。
pub mod agent;
pub mod app;
pub mod embed;
pub mod infra;
pub mod protocol;
pub(crate) mod utils;

// ── Curated `pub use` (c500 / architecture.ar09) ─────────────────────

pub use crate::app::core::attach::{
    DEFAULT_ATTACH_URL, attach_fail_message, attach_preflight_with, probe_host, resolve_attach_url,
};
#[cfg(feature = "server")]
pub use crate::app::core::driver::{LinkHealth, LinkTunings, XyRemoteDriver};
pub use crate::app::core::driver::{XyDriver, XyDriverError, XyInProcessDriver};
#[cfg(feature = "server")]
pub use crate::app::core::host_client::HttpWsClient;
pub use crate::app::core::host_client::{HostClient, HostClientError, InProcessClient, MuxStream};
/// att ex 族：无头导出/导入协作者（BDD 直驱；真身 app::core 私有链）。
pub use crate::app::core::session_export::SessionExporter;
pub use crate::protocol::error::{
    XyError, XyExportError, XySessionError, XySessionStoreError, XyToolError, XyTrustError,
};
pub use crate::protocol::lifecycle::{XyEvent, XyEventError};
pub use crate::protocol::model::{XyChunk, XyModelMeta, XyToolSchema};
pub use crate::protocol::model::{XyModelConfig, XyModelKind};
pub use crate::protocol::{
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
