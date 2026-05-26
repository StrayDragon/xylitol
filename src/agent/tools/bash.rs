use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::time::timeout;

use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};

pub(crate) struct BashTool;

const MAX_OUTPUT_SIZE: usize = 1_048_576;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 120;

#[async_trait]
impl XyTool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command on the local system. Use with caution."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn execute(&self, _ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let cmd = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| XyToolError::InvalidArgs("missing required argument: command".into()))?;

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
        .map_err(|_| XyToolError::Timeout(timeout_duration))?
        .map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to execute command: {}", e))
        })?;

        let stdout = truncate_output(&output.stdout);
        let stderr = truncate_output(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        #[cfg(feature = "infra-rtk")]
        let stdout = compress_with_rtk(&stdout);

        Ok(serde_json::to_string(&json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        }))
        .unwrap())
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

#[cfg(feature = "infra-rtk")]
fn compress_with_rtk(output: &str) -> String {
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

    fn test_ctx() -> XyToolCtx {
        XyToolCtx {
            call_id: "test-call".into(),
        }
    }

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool;
        let result = tool
            .execute(&test_ctx(), json!({ "command": "echo hello" }))
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
            .execute(&test_ctx(), json!({ "command": "exit 42" }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["exit_code"], 42);
    }

    #[tokio::test]
    async fn test_bash_stderr() {
        let tool = BashTool;
        let result = tool
            .execute(&test_ctx(), json!({ "command": "echo error >&2" }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert!(v["stderr"].as_str().unwrap().contains("error"));
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let tool = BashTool;
        let result = tool
            .execute(&test_ctx(), json!({ "command": "sleep 10", "timeout": 1 }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bash_missing_command() {
        let tool = BashTool;
        let result = tool.execute(&test_ctx(), json!({})).await;
        assert!(result.is_err());
    }
}
