pub mod embed;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "server")]
pub use xylitol_kernel::attach::attach_preflight_with;
pub use xylitol_kernel::attach::{
    DEFAULT_ATTACH_URL, attach_fail_message, probe_host, resolve_attach_url,
};
pub use xylitol_kernel::driver::SessionExporter;
pub use xylitol_kernel::driver::{XyDriver, XyDriverError, XyInProcessDriver};
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

#[cfg(feature = "cli")]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    cli::run().await
}

#[cfg(all(test, feature = "server"))]
#[path = "../tests/occupancy/mod.rs"]
mod occupancy_tests;

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
