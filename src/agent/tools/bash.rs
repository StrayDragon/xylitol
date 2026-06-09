//! Bash tool — executes shell commands with abort and timeout support.
//!
//! Key behaviors (aligns with pi's bash.ts):
//! - CancellationToken kills the process tree
//! - Merges stdout/stderr streaming
//! - Configurable timeout (default 30s, max 120s)
//! - Output truncated to DEFAULT_MAX_BYTES
//! - Shell detection (sh on Linux, fallback to bash)

use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::time::timeout;

use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};

use super::truncate::{DEFAULT_MAX_BYTES, TruncationOptions, format_size, truncate_tail};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 120;

pub struct BashTool;

#[async_trait]
impl XyTool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command on the local system. Supports timeout and abort. Use with caution."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "description": {
                    "type": "string",
                    "description": "Human-readable description of what this command does"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default 30, max 120)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let cmd = args["command"]
            .as_str()
            .ok_or_else(|| XyToolError::InvalidArgs("missing 'command'".into()))?;

        let requested = args["timeout"]
            .as_i64()
            .unwrap_or(DEFAULT_TIMEOUT_SECS as i64);
        let timeout_secs = if requested <= 0 {
            DEFAULT_TIMEOUT_SECS
        } else {
            (requested as u64).min(MAX_TIMEOUT_SECS)
        };

        let timeout_dur = Duration::from_secs(timeout_secs);

        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        // Detect available shell
        let shell = if is_executable("bash") { "bash" } else { "sh" };

        let cancel = ctx.cancel.clone();
        let child = Command::new(shell)
            .arg("-c")
            .arg(cmd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("spawn failed: {e}")))?;

        let pid = child.id().unwrap_or(0);

        let output_fut = child.wait_with_output();

        let child_result = tokio::select! {
            _ = cancel.cancelled() => {
                // Kill process tree via a separately-tracked pid
                kill_tree(pid).await;
                // We can't access child here but we already have pid
                return Err(XyToolError::Aborted);
            }
            r = timeout_fallible(output_fut, timeout_dur) => r,
        };

        let output = match child_result {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "wait error: {e}"
                )));
            }
            Err(_elapsed) => {
                // Timeout — kill via pid (child already moved)
                kill_tree(pid).await;
                return Err(XyToolError::Timeout(timeout_dur));
            }
        };

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();

        // Merge stdout + stderr like pi does (stderr prepended for visibility)
        let combined = if stderr_raw.is_empty() {
            stdout_raw.clone()
        } else if stdout_raw.is_empty() {
            stderr_raw.clone()
        } else {
            format!("{stderr_raw}{stdout_raw}")
        };

        // Truncate tail (show end of output — errors/final results)
        let truncation = truncate_tail(
            &combined,
            TruncationOptions {
                max_lines: None,
                max_bytes: Some(DEFAULT_MAX_BYTES),
            },
        );

        let mut output_text = truncation.content;
        if truncation.truncated {
            output_text.push_str(&format!(
                "\n[Output truncated at {}]",
                format_size(DEFAULT_MAX_BYTES)
            ));
        }

        Ok(serde_json::to_string(&json!({
            "stdout": stdout_raw,
            "stderr": stderr_raw,
            "exit_code": exit_code,
            "combined": output_text,
        }))
        .unwrap())
    }
}

fn is_executable(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn kill_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    // Kill process group
    let _ = Command::new("kill")
        .args(["-9", &format!("-{pid}")])
        .output()
        .await;
}

async fn timeout_fallible<F, T, E>(
    fut: F,
    dur: Duration,
) -> Result<Result<T, E>, tokio::time::error::Elapsed>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    match timeout(dur, fut).await {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> XyToolCtx {
        XyToolCtx::new("test-call")
    }

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool;
        let result = tool
            .execute(&test_ctx(), json!({"command": "echo hello"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["exit_code"], 0);
        assert!(v["stdout"].as_str().unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_exit_code_nonzero() {
        let tool = BashTool;
        let result = tool
            .execute(&test_ctx(), json!({"command": "exit 42"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["exit_code"], 42);
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let tool = BashTool;
        let result = tool
            .execute(&test_ctx(), json!({"command": "sleep 10", "timeout": 1}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bash_missing_command() {
        let tool = BashTool;
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
    }
}
