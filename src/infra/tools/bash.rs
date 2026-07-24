//! Bash tool — executes shell commands with abort and timeout support.
//!
//! Key behaviors (aligns with pi's bash.ts):
//! - CancellationToken kills the process tree
//! - Merges stdout/stderr streaming
//! - Optional timeout (default unlimited, max 120s) with graduated escalation
//! - Output truncated to DEFAULT_MAX_BYTES
//! - Cross-platform shell discovery via `infra::process::shell`

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::time::timeout;

use crate::protocol::error::XyToolError;
use crate::protocol::ports::XyToolCtx;
use crate::protocol::{ToolTimeout, ToolTimeoutError};

use super::accumulator::OutputAccumulator;
use super::typed::TypedTool;

const SIGTERM_GRACE_SECS: u64 = 5;

#[derive(Debug, Deserialize)]
pub struct BashArgs {
    command: String,
    #[serde(default)]
    #[allow(dead_code)] // accepted in schema for LLM UX; not used by executor
    description: Option<String>,
    /// Optional seconds; omit for unlimited. Zero/negative are invalid.
    #[serde(default)]
    timeout: Option<i64>,
}

// ── BashOperations trait ──────────────────────────────────────────────

/// Abstract interface for bash execution.
///
/// Supports mock implementations for testing and hook injection.
#[async_trait]
#[allow(dead_code)] // test-only construction seam (MockBash in cfg(test))
pub trait BashOperations: Send + Sync {
    /// Execute a shell command and return its output.
    async fn execute(
        &self,
        command: &str,
        tool_timeout: ToolTimeout,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<BashOutput, BashError>;
}

/// Result of a bash execution.
#[derive(Debug, Clone)]
pub struct BashOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub combined: String,
    pub truncated: bool,
    pub full_output_path: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BashError {
    SpawnFailed(String),
    Timeout,
    Aborted,
}

// ── Hooks ────────────────────────────────────────────────────────────

/// Hooks that fire before and after bash execution.
#[derive(Default, Clone)]
#[allow(clippy::type_complexity)]
pub struct BashHooks {
    /// Called with the command string just before spawning.
    pub pre_spawn: Option<Arc<dyn Fn(&str) + Send + Sync>>,
    /// Called with the result after execution completes.
    pub post_spawn: Option<Arc<dyn Fn(&BashOutput) + Send + Sync>>,
}

// ── RealBashOperations ────────────────────────────────────────────────

/// Real implementation using system shell.
#[derive(Default, Clone)]
pub struct RealBashOperations {
    pub hooks: BashHooks,
}

#[async_trait]
impl BashOperations for RealBashOperations {
    async fn execute(
        &self,
        command: &str,
        tool_timeout: ToolTimeout,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<BashOutput, BashError> {
        // Pre-spawn hook
        if let Some(ref hook) = self.hooks.pre_spawn {
            hook(command);
        }

        if cancel.is_cancelled() {
            return Err(BashError::Aborted);
        }

        // Use c135's shell discovery
        let shell_cfg = crate::infra::process::shell::find_bash(None);

        let child = Command::new(&shell_cfg.shell)
            .args(&shell_cfg.args)
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| BashError::SpawnFailed(e.to_string()))?;

        let pid = child.id().unwrap_or(0);
        let output_fut = child.wait_with_output();

        // Graduated timeout when limited: SIGTERM → 5s grace → SIGKILL
        let output = tokio::select! {
            _ = cancel.cancelled() => {
                sigterm_then_sigkill(pid).await;
                return Err(BashError::Aborted);
            }
            r = graduated_timeout(output_fut, tool_timeout.duration(), pid) => r,
        };

        let output = match output {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return Err(BashError::SpawnFailed(e.to_string())),
            Err(()) => return Err(BashError::Timeout),
        };

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();

        // Feed into OutputAccumulator for rolling buffer + temp file support
        let mut acc = OutputAccumulator::new();
        if !stderr_raw.is_empty() {
            acc.append(stderr_raw.as_bytes());
        }
        if !stdout_raw.is_empty() {
            acc.append(stdout_raw.as_bytes());
        }
        let snapshot = acc.finish();
        let display = snapshot.display_content();

        let bash_output = BashOutput {
            // Keep fields for hooks/tests, but never exceed truncated display size
            // when spilled (full bytes live only in full_output_path).
            stdout: display.clone(),
            stderr: String::new(),
            exit_code,
            combined: display,
            truncated: snapshot.truncated,
            full_output_path: snapshot
                .full_output_path
                .and_then(|p| p.to_str().map(|s| s.to_string())),
        };

        // Post-spawn hook
        if let Some(ref hook) = self.hooks.post_spawn {
            hook(&bash_output);
        }

        Ok(bash_output)
    }
}

// ── Graduated timeout ────────────────────────────────────────────────

/// Send SIGTERM first, then SIGKILL after grace period.
async fn sigterm_then_sigkill(pid: u32) {
    if pid == 0 {
        return;
    }

    // Send SIGTERM
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-15", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    // Wait grace period
    tokio::time::sleep(Duration::from_secs(SIGTERM_GRACE_SECS)).await;

    // Send SIGKILL if still running
    crate::infra::process::group::kill_process_tree(pid);
}

/// Wait; when `dur` is Some, graduated timeout: first timeout → SIGTERM → grace → SIGKILL.
async fn graduated_timeout<F, T>(
    fut: F,
    dur: Option<Duration>,
    pid: u32,
) -> Result<Result<T, std::io::Error>, ()>
where
    F: std::future::Future<Output = Result<T, std::io::Error>>,
{
    match dur {
        None => Ok(fut.await),
        Some(d) => match timeout(d, fut).await {
            Ok(result) => Ok(result),
            Err(_elapsed) => {
                sigterm_then_sigkill(pid).await;
                Err(())
            }
        },
    }
}

// ── BashTool (existing XyTool impl) ───────────────────────────────────

pub struct BashTool {
    operations: Arc<dyn BashOperations>,
}

impl Default for BashTool {
    fn default() -> Self {
        Self {
            operations: Arc::new(RealBashOperations::default()),
        }
    }
}

impl BashTool {
    #[allow(dead_code)] // test-only construction seam
    pub fn with_operations(ops: impl BashOperations + 'static) -> Self {
        Self {
            operations: Arc::new(ops),
        }
    }

    #[allow(dead_code)] // test-only construction seam
    pub fn with_hooks(hooks: BashHooks) -> Self {
        Self {
            operations: Arc::new(RealBashOperations { hooks }),
        }
    }
}

#[async_trait]
impl TypedTool for BashTool {
    type Args = BashArgs;

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
                    "description": "Optional timeout in seconds (omit for unlimited; max 120). Zero is invalid."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: BashArgs) -> Result<String, XyToolError> {
        let BashArgs {
            command: cmd,
            description: _,
            timeout: requested,
        } = args;

        let tool_timeout = ToolTimeout::from_i64_opt(requested).map_err(|e| match e {
            ToolTimeoutError::ZeroOrNegative | ToolTimeoutError::AboveMax { .. } => {
                XyToolError::InvalidArgs(e.to_string())
            }
        })?;

        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        // Live uplink path: stream stdout/stderr chunks to ReAct while running
        // (c1255 ToolExecutionUpdate). Falls back to wait_with_output otherwise.
        if let Some(out_tx) = ctx.output_tx.clone() {
            return self
                .execute_streaming(&cmd, tool_timeout, ctx.cancel.clone(), out_tx)
                .await;
        }

        let output = self
            .operations
            .execute(&cmd, tool_timeout, ctx.cancel.clone())
            .await
            .map_err(|e| match e {
                BashError::Aborted => XyToolError::Aborted,
                BashError::Timeout => XyToolError::Timeout(
                    tool_timeout
                        .duration()
                        .unwrap_or_else(|| Duration::from_secs(0)),
                ),
                BashError::SpawnFailed(msg) => {
                    XyToolError::ExecutionFailed(anyhow::anyhow!("spawn failed: {msg}"))
                }
            })?;

        Ok(serde_json::to_string(&json!({
            "stdout": output.combined,
            "stderr": "",
            "exit_code": output.exit_code,
            "combined": output.combined,
            "truncated": output.truncated,
            "full_output_path": output.full_output_path,
        }))
        .expect("serde_json::to_string on Value/Map never fails"))
    }

    fn prompt_guidelines(&self) -> &[&str] {
        &[
            "Prefer specialized read/edit/write tools for file operations; use bash for shell commands.",
        ]
    }
}

impl BashTool {
    async fn execute_streaming(
        &self,
        cmd: &str,
        tool_timeout: ToolTimeout,
        cancel: tokio_util::sync::CancellationToken,
        out_tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, XyToolError> {
        use crate::infra::bash_exec::InfraBashExecutor;
        use crate::protocol::ports::{BashExecOpts, XyBashExecutor};

        let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
        let forward = tokio::spawn(async move {
            while let Some(bytes) = chunk_rx.recv().await {
                let s = String::from_utf8_lossy(&bytes).into_owned();
                if out_tx.send(s).await.is_err() {
                    break;
                }
            }
        });

        let result = InfraBashExecutor::new()
            .execute(
                cmd,
                BashExecOpts {
                    cancel: Some(cancel),
                    chunk_tx: Some(chunk_tx),
                    timeout: tool_timeout,
                },
            )
            .await;

        let _ = forward.await;

        if result.cancelled {
            return Err(XyToolError::Aborted);
        }
        if result.timed_out {
            return Err(XyToolError::Timeout(
                tool_timeout
                    .duration()
                    .unwrap_or_else(|| Duration::from_secs(0)),
            ));
        }

        Ok(serde_json::to_string(&json!({
            "stdout": result.output,
            "stderr": "",
            "exit_code": result.exit_code,
            "combined": result.output,
            "full_output_path": result.full_output_path,
            "truncated": result.truncated,
        }))
        .expect("serde_json::to_string on Value/Map never fails"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ports::XyTool;

    fn test_ctx() -> XyToolCtx {
        XyToolCtx::new("test-call")
    }

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool::default();
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
        let tool = BashTool::default();
        let result = tool
            .execute(&test_ctx(), json!({"command": "exit 42"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["exit_code"], 42);
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let tool = BashTool::default();
        let result = tool
            .execute(&test_ctx(), json!({"command": "sleep 10", "timeout": 1}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bash_missing_command() {
        let tool = BashTool::default();
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
    }

    #[tokio::test]
    async fn test_bash_operations_trait_mock() {
        // Mock implementation for testing
        struct MockBash;

        #[async_trait]
        impl BashOperations for MockBash {
            async fn execute(
                &self,
                _command: &str,
                _tool_timeout: ToolTimeout,
                _cancel: tokio_util::sync::CancellationToken,
            ) -> Result<BashOutput, BashError> {
                Ok(BashOutput {
                    stdout: "mock output".into(),
                    stderr: String::new(),
                    exit_code: 0,
                    combined: "mock output".into(),
                    truncated: false,
                    full_output_path: None,
                })
            }
        }

        let tool = BashTool::with_operations(MockBash);
        let result = tool
            .execute(&test_ctx(), json!({"command": "anything"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert!(v["stdout"].as_str().unwrap().contains("mock"));
    }

    #[tokio::test]
    async fn test_bash_hooks_get_called() {
        let pre_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let post_called = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let pre = pre_called.clone();
        let post = post_called.clone();

        let hooks = BashHooks {
            pre_spawn: Some(Arc::new(move |_cmd: &str| {
                pre.store(true, std::sync::atomic::Ordering::SeqCst);
            })),
            post_spawn: Some(Arc::new(move |_result: &BashOutput| {
                post.store(true, std::sync::atomic::Ordering::SeqCst);
            })),
        };

        let tool = BashTool::with_hooks(hooks);
        let _ = tool
            .execute(&test_ctx(), json!({"command": "echo hook-test"}))
            .await;

        assert!(pre_called.load(std::sync::atomic::Ordering::SeqCst));
        assert!(post_called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_bash_omit_timeout_unlimited() {
        let tool = BashTool::default();
        let result = tool
            .execute(&test_ctx(), json!({"command": "sleep 2"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["exit_code"], 0);
    }

    #[tokio::test]
    async fn test_bash_zero_timeout_rejected() {
        let tool = BashTool::default();
        let err = tool
            .execute(&test_ctx(), json!({"command": "echo hi", "timeout": 0}))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("invalid timeout")
                || matches!(err, XyToolError::InvalidArgs(_)),
            "got {err}"
        );
    }

    #[tokio::test]
    async fn test_bash_streaming_respects_timeout() {
        let tool = BashTool::default();
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(8);
        let ctx = XyToolCtx::new("stream").with_output_tx(tx);
        let err = tool
            .execute(&ctx, json!({"command": "sleep 10", "timeout": 1}))
            .await
            .unwrap_err();
        assert!(matches!(err, XyToolError::Timeout(_)), "got {err}");
    }
}
