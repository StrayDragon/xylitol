//! Runtime boundary for bash command execution.

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

/// Result of executing a bash command.
#[derive(Debug, Clone, Default)]
pub struct BashResult {
    /// Combined stdout + stderr output (possibly truncated; tail kept).
    pub output: String,
    /// Process exit code (`None` if killed/cancelled before exit).
    pub exit_code: Option<i32>,
    /// Whether the command was cancelled via the cancel token.
    pub cancelled: bool,
    /// Whether the output was truncated.
    pub truncated: bool,
    /// Path to a temp file containing the full output, if spilled.
    pub full_output_path: Option<String>,
}

/// Bash executor port — abstracts process spawn + output streaming so the
/// agent need not depend on infra exec primitives.
#[async_trait]
pub trait BashExecutor: Send + Sync {
    /// Execute a bash command with an optional cancellation token.
    async fn execute(&self, command: &str, cancel: Option<CancellationToken>) -> BashResult;
}
