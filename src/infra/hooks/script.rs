//! Hook script execution — spawn, stdin JSON, stdout parsing, timeout.
//!
//! Protocol:
//! ```text
//! stdin:  {"event":"pre.tool_call","tool":"bash",...}
//! stdout: {"action":"block","reason":"..."}  (optional)
//! ```
//! Empty stdout = [`HookAction::Allow`].

use std::collections::HashMap;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{debug, warn};

use super::{HookAction, HookEvent, HookPhase};

/// Run a single hook script and return its action.
///
/// Spawns `command` as a subprocess, pipes the event JSON context to stdin,
/// and reads stdout for a control directive. If the process exits without
/// producing output, returns [`HookAction::Allow`].
///
/// On timeout, the process is killed and `Allow` is returned (non-blocking
/// default to avoid freezing the agent).
pub async fn run_hook_script(
    command: &str,
    event: &HookEvent,
    phase: HookPhase,
    timeout: Duration,
    env: &HashMap<String, String>,
) -> HookAction {
    let ctx = event.to_json_context(phase);
    let ctx_bytes = serde_json::to_vec(&ctx).unwrap_or_default();

    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(command)
        .env_clear()
        .envs(env)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            warn!(command = command, error = %e, "Failed to spawn hook script");
            return HookAction::Allow;
        }
    };

    // Write context JSON to stdin, then close it so the child can read EOF.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&ctx_bytes).await;
        let _ = stdin.shutdown().await;
    }

    // Wait for output with timeout. On timeout, kill the child to prevent zombies.
    let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let trimmed = stdout.trim();
                if trimmed.is_empty() {
                    return HookAction::Allow;
                }
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(val) => HookAction::from_json(&val),
                    Err(e) => {
                        debug!(
                            command = command,
                            error = %e,
                            stdout = trimmed,
                            "Hook stdout is not valid JSON, treating as allow"
                        );
                        HookAction::Allow
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    command = command,
                    exit_code = output.status.code(),
                    stderr = stderr.as_ref(),
                    "Hook script exited with non-zero status"
                );
                HookAction::Allow
            }
        }
        Ok(Err(e)) => {
            warn!(command = command, error = %e, "Hook script I/O error");
            HookAction::Allow
        }
        Err(_elapsed) => {
            warn!(
                command = command,
                timeout_ms = timeout.as_millis(),
                "Hook script timed out, blocking (fail-closed)"
            );
            HookAction::Block {
                reason: format!("hook timed out after {}ms", timeout.as_millis()),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_env() -> HashMap<String, String> {
        HashMap::new()
    }

    #[tokio::test]
    async fn test_echo_allow() {
        let action = run_hook_script(
            "echo '{\"action\":\"allow\"}'",
            &HookEvent::ToolCall {
                tool: "bash".into(),
                args: serde_json::json!({}),
            },
            HookPhase::Pre,
            Duration::from_secs(5),
            &make_env(),
        )
        .await;
        assert!(matches!(action, HookAction::Allow));
    }

    #[tokio::test]
    async fn test_echo_block() {
        let action = run_hook_script(
            "echo '{\"action\":\"block\",\"reason\":\"no\"}'",
            &HookEvent::ToolCall {
                tool: "bash".into(),
                args: serde_json::json!({}),
            },
            HookPhase::Pre,
            Duration::from_secs(5),
            &make_env(),
        )
        .await;
        match action {
            HookAction::Block { reason } => assert_eq!(reason, "no"),
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_empty_stdout_is_allow() {
        let action = run_hook_script(
            "echo ''",
            &HookEvent::ToolCall {
                tool: "bash".into(),
                args: serde_json::json!({}),
            },
            HookPhase::Pre,
            Duration::from_secs(5),
            &make_env(),
        )
        .await;
        assert!(matches!(action, HookAction::Allow));
    }

    #[tokio::test]
    async fn test_no_output_is_allow() {
        let action = run_hook_script(
            "true",
            &HookEvent::ToolCall {
                tool: "bash".into(),
                args: serde_json::json!({}),
            },
            HookPhase::Pre,
            Duration::from_secs(5),
            &make_env(),
        )
        .await;
        assert!(matches!(action, HookAction::Allow));
    }

    #[tokio::test]
    async fn test_nonzero_exit_is_allow() {
        let action = run_hook_script(
            "exit 1",
            &HookEvent::ToolCall {
                tool: "bash".into(),
                args: serde_json::json!({}),
            },
            HookPhase::Pre,
            Duration::from_secs(5),
            &make_env(),
        )
        .await;
        assert!(
            matches!(action, HookAction::Allow),
            "non-zero exit should be allow, got {action:?}"
        );
    }

    #[tokio::test]
    async fn test_timeout_is_block() {
        let action = run_hook_script(
            "sleep 10",
            &HookEvent::ToolCall {
                tool: "bash".into(),
                args: serde_json::json!({}),
            },
            HookPhase::Pre,
            Duration::from_millis(50),
            &make_env(),
        )
        .await;
        assert!(
            matches!(action, HookAction::Block { .. }),
            "timeout should be block (fail-closed), got {action:?}"
        );
    }

    #[tokio::test]
    async fn test_bogus_stdout_is_allow() {
        let action = run_hook_script(
            "echo 'not json'",
            &HookEvent::ToolCall {
                tool: "bash".into(),
                args: serde_json::json!({}),
            },
            HookPhase::Pre,
            Duration::from_secs(5),
            &make_env(),
        )
        .await;
        assert!(matches!(action, HookAction::Allow));
    }
}
