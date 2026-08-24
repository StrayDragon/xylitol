//! Shared process helpers for external-process tools (bash, grep, find).
//!
//! Delegates to [`crate::infra::process`] for cross-platform implementation.

use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::protocol::error::XyToolError;
use crate::protocol::{ToolTimeout, ToolTimeoutError};

/// Kill a process tree by process group ID.
///
/// Uses `kill -9 -<pid>` on Unix, `taskkill /F /T` on Windows.
pub(crate) async fn kill_tree(pid: u32) {
    crate::infra::process::group::kill_process_tree(pid);
}

/// Parse the shared `timeout` tool argument: validate → default → clamp.
pub(crate) fn parse_tool_timeout(
    raw: Option<i64>,
    default_secs: u64,
) -> Result<ToolTimeout, XyToolError> {
    let timeout = ToolTimeout::from_i64_opt(raw).map_err(|e| match e {
        ToolTimeoutError::ZeroOrNegative => XyToolError::InvalidArgs(e.to_string()),
    })?;
    Ok(timeout.or_default(default_secs).clamped())
}

/// Spawn an external search tool (`rg` / `fd`) with piped stdio and wait under
/// cancel + deadline supervision; the process tree is killed when either fires.
pub(crate) async fn run_search_tool(
    program: &str,
    args: &[String],
    cancel: CancellationToken,
    tool_timeout: ToolTimeout,
) -> Result<std::process::Output, XyToolError> {
    let child = Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("spawn {program}: {e}")))?;

    let pid = child.id().unwrap_or(0);
    let deadline = tool_timeout
        .duration()
        .map(|d| tokio::time::Instant::now() + d);

    let child_result = tokio::select! {
        _ = cancel.cancelled() => {
            kill_tree(pid).await;
            return Err(XyToolError::Aborted);
        }
        result = child.wait_with_output() => result,
        _ = async {
            match deadline {
                Some(dl) => tokio::time::sleep_until(dl).await,
                None => std::future::pending::<()>().await,
            }
        } => {
            kill_tree(pid).await;
            return Err(XyToolError::Timeout(
                tool_timeout
                    .duration()
                    .expect("timeout arm only fires when limited"),
            ));
        }
    };

    child_result.map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("wait {program}: {e}")))
}

/// Map a non-success external-tool exit to `ExecutionFailed`, preferring stderr.
pub(crate) fn external_tool_failure(output: &std::process::Output, program: &str) -> XyToolError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let msg = if !stderr.is_empty() {
        stderr.to_string()
    } else {
        format!("{program} exited with {}", output.status)
    };
    XyToolError::ExecutionFailed(anyhow::anyhow!("{msg}"))
}
