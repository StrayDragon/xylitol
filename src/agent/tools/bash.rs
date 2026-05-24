use adk_core::{AdkError, ErrorCategory, ErrorComponent, Result, Tool, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub(crate) struct BashTool;

/// Maximum output size in bytes (1 MB).
const MAX_OUTPUT_SIZE: usize = 1_048_576;

/// Default timeout for bash commands in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Hard cap for bash timeout (matches security.bash.timeout_secs default).
const MAX_TIMEOUT_SECS: u64 = 120;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command on the local system. Use with caution."
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default 30)"
                },
                "description": {
                    "type": "string",
                    "description": "Human-readable description of what this command does"
                }
            },
            "required": ["command"]
        }))
    }

    async fn execute(&self, _ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
        let cmd = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::InvalidInput,
                    "bash.missing_command",
                    "missing required argument: command",
                )
            })?;

        let requested = args
            .get("timeout")
            .and_then(|v| v.as_i64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS as i64);

        let timeout_secs = if requested <= 0 {
            DEFAULT_TIMEOUT_SECS
        } else {
            (requested as u64).min(MAX_TIMEOUT_SECS)
        };

        let timeout_duration = Duration::from_secs(timeout_secs);

        let output = timeout(
            timeout_duration,
            Command::new("sh").arg("-c").arg(cmd).output(),
        )
        .await
        .map_err(|_| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::Timeout,
                "bash.timeout",
                format!("command timed out after {}s", timeout_secs),
            )
        })?
        .map_err(|e| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::Internal,
                "bash.execution_failed",
                format!("failed to execute command: {}", e),
            )
        })?;

        let stdout = truncate_output(&output.stdout);
        let stderr = truncate_output(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        // RTK compression when feature is enabled
        #[cfg(feature = "infra-rtk")]
        let stdout = compress_with_rtk(&stdout);

        Ok(json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        }))
    }
}

fn truncate_output(data: &[u8]) -> String {
    let text = String::from_utf8_lossy(data);
    if text.len() > MAX_OUTPUT_SIZE {
        let truncated = &text[..text.floor_char_boundary(MAX_OUTPUT_SIZE)];
        format!(
            "{}...\n[Output truncated at {} bytes]",
            truncated, MAX_OUTPUT_SIZE
        )
    } else {
        text.to_string()
    }
}

/// Compress bash output using the rtk proxy pipeline.
/// Only compiled when `infra-rtk` feature is enabled.
#[cfg(feature = "infra-rtk")]
fn compress_with_rtk(output: &str) -> String {
    // RTK compression pipes output through the rtk binary for token optimization.
    // If rtk is not available, fall back to original output.
    use std::process::Command as StdCommand;

    let result = StdCommand::new("rtk")
        .arg("compress")
        .arg("--text")
        .arg(output)
        .output();

    match result {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => output.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_context() -> Arc<dyn ToolContext> {
        crate::agent::tools::patch::mock_context()
    }

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool;
        let result = tool
            .execute(test_context(), json!({ "command": "echo hello" }))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 0);
        let stdout = result["stdout"].as_str().unwrap();
        assert!(stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_exit_code_nonzero() {
        let tool = BashTool;
        let result = tool
            .execute(test_context(), json!({ "command": "exit 42" }))
            .await
            .unwrap();

        assert_eq!(result["exit_code"], 42);
    }

    #[tokio::test]
    async fn test_bash_stderr() {
        let tool = BashTool;
        let result = tool
            .execute(test_context(), json!({ "command": "echo error >&2" }))
            .await
            .unwrap();

        let stderr = result["stderr"].as_str().unwrap();
        assert!(stderr.contains("error"));
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let tool = BashTool;
        let result = tool
            .execute(
                test_context(),
                json!({ "command": "sleep 10", "timeout": 1 }),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bash_missing_command() {
        let tool = BashTool;
        let result = tool.execute(test_context(), json!({})).await;
        assert!(result.is_err());
    }
}
