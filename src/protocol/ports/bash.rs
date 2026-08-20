//! Runtime boundary for bash command execution.

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::protocol::ToolTimeout;

/// Result of executing a bash command.
#[derive(Debug, Clone, Default)]
pub struct XyBashResult {
    /// Combined stdout + stderr output (possibly truncated; tail kept).
    pub output: String,
    /// Process exit code (`None` if killed/cancelled before exit).
    pub exit_code: Option<i32>,
    /// Whether the command was cancelled via the cancel token.
    pub cancelled: bool,
    /// Whether the command hit a wall-clock timeout.
    pub timed_out: bool,
    /// Whether the output was truncated.
    pub truncated: bool,
    /// Path to a temp file containing the full output, if spilled.
    pub full_output_path: Option<String>,
}

/// Options for [`XyBashExecutor::execute`].
///
/// `chunk_tx`: when `Some`, the executor emits output byte chunks on this
/// bounded channel (`try_send` + sender-side coalesce on Full). When `None`,
/// output is accumulated silently (print / non-streaming callers).
#[derive(Default)]
pub struct BashExecOpts {
    /// Optional cancellation token (kill process tree when cancelled).
    pub cancel: Option<CancellationToken>,
    /// Optional bounded channel for live output bytes.
    pub chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
    /// Wall-clock timeout; default [`ToolTimeout::Unlimited`].
    pub timeout: ToolTimeout,
    /// Working directory for the spawned shell; `None` inherits the process
    /// cwd. Callers that own a session workspace (attach writer) MUST pass it
    /// so `!cmd` runs in the workspace, not the server process directory.
    pub cwd: Option<PathBuf>,
}

impl BashExecOpts {
    /// Cancel only (no live chunks, unlimited timeout).
    pub fn cancel_only(cancel: CancellationToken) -> Self {
        Self {
            cancel: Some(cancel),
            chunk_tx: None,
            timeout: ToolTimeout::Unlimited,
            cwd: None,
        }
    }
}

/// Bash executor port — abstracts process spawn + output streaming so the
/// agent need not depend on infra exec primitives.
#[async_trait]
pub trait XyBashExecutor: Send + Sync {
    /// Execute a bash command with cancel + optional chunk uplink.
    async fn execute(&self, command: &str, opts: BashExecOpts) -> XyBashResult;
}
