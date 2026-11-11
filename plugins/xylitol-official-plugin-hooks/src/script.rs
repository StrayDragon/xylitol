use std::collections::HashMap;
use std::time::Duration;

use log::{debug, warn};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use xylitol_plugin_sdk::host_env;

use super::{HookAction, HookEvent, HookPhase};

#[derive(Debug, Clone)]
pub enum HookScriptFailure {
    SpawnFailed(String),
    NonZeroExit { code: Option<i32>, stderr: String },
    BadJson(String),
    IoError(String),
    Timeout(Duration),
}

impl HookScriptFailure {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SpawnFailed(_) => "spawn_failed",
            Self::NonZeroExit { .. } => "nonzero_exit",
            Self::BadJson(_) => "bad_json",
            Self::IoError(_) => "io_error",
            Self::Timeout(_) => "timeout",
        }
    }

    pub fn reason(&self) -> String {
        match self {
            Self::SpawnFailed(error) => format!("hook failed (spawn_failed): {error}"),
            Self::NonZeroExit { code, stderr } => {
                format!("hook failed (nonzero_exit, code {code:?}): {stderr}")
            }
            Self::BadJson(error) => format!("hook failed (bad_json): {error}"),
            Self::IoError(error) => format!("hook failed (io_error): {error}"),
            Self::Timeout(duration) => {
                format!("hook failed (timeout after {}ms)", duration.as_millis())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum HookScriptResult {
    Action(HookAction),
    Failed(HookScriptFailure),
}

pub async fn run_hook_script(
    command: &str,
    event: &HookEvent,
    phase: HookPhase,
    timeout: Option<Duration>,
    env: &HashMap<String, String>,
) -> HookScriptResult {
    let ctx = event.to_json_context(phase);
    run_hook_script_with_context(command, &ctx, timeout, env).await
}

pub async fn run_hook_script_with_context(
    command: &str,
    context: &serde_json::Value,
    timeout: Option<Duration>,
    env: &HashMap<String, String>,
) -> HookScriptResult {
    let ctx_bytes = serde_json::to_vec(context).unwrap_or_default();

    let mut spawn = Command::new("sh");
    spawn
        .arg("-c")
        .arg(command)
        .env_clear()
        .envs(env)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());
    host_env::scrub_tokio_command(&mut spawn);
    let mut child = match spawn.spawn() {
        Ok(c) => c,
        Err(e) => {
            warn!(
                "Failed to spawn hook script command={} error={}",
                command, e
            );
            return HookScriptResult::Failed(HookScriptFailure::SpawnFailed(e.to_string()));
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&ctx_bytes).await;
        let _ = stdin.shutdown().await;
    }

    let wait = child.wait_with_output();
    let result = match timeout {
        Some(d) => match tokio::time::timeout(d, wait).await {
            Ok(inner) => Ok(inner),
            Err(_elapsed) => Err(d),
        },
        None => Ok(wait.await),
    };

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let trimmed = stdout.trim();
                if trimmed.is_empty() {
                    return HookScriptResult::Action(HookAction::Allow);
                }
                match serde_json::from_str::<serde_json::Value>(trimmed) {
                    Ok(val) => HookScriptResult::Action(HookAction::from_json(&val)),
                    Err(e) => {
                        debug!(
                            "Hook stdout is not valid JSON command={} error={} stdout={}",
                            command, e, trimmed
                        );
                        HookScriptResult::Failed(HookScriptFailure::BadJson(e.to_string()))
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                warn!(
                    "Hook script exited with non-zero status command={command} exit_code={:?} stderr={stderr}",
                    output.status.code()
                );
                HookScriptResult::Failed(HookScriptFailure::NonZeroExit {
                    code: output.status.code(),
                    stderr,
                })
            }
        }
        Ok(Err(e)) => {
            warn!("Hook script I/O error command={} error={}", command, e);
            HookScriptResult::Failed(HookScriptFailure::IoError(e.to_string()))
        }
        Err(duration) => {
            let ms = duration.as_millis();
            warn!(
                "Hook script timed out command={} timeout_ms={}",
                command, ms
            );
            HookScriptResult::Failed(HookScriptFailure::Timeout(duration))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_env() -> HashMap<String, String> {
        HashMap::new()
    }

    fn tool_call_event() -> HookEvent {
        HookEvent::ToolCall {
            tool: "bash".into(),
            args: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn test_echo_allow() {
        let result = run_hook_script(
            "echo '{\"action\":\"allow\"}'",
            &tool_call_event(),
            HookPhase::Pre,
            Some(Duration::from_secs(5)),
            &make_env(),
        )
        .await;
        assert!(matches!(
            result,
            HookScriptResult::Action(HookAction::Allow)
        ));
    }

    #[tokio::test]
    async fn test_echo_block() {
        let result = run_hook_script(
            "echo '{\"action\":\"block\",\"reason\":\"no\"}'",
            &tool_call_event(),
            HookPhase::Pre,
            Some(Duration::from_secs(5)),
            &make_env(),
        )
        .await;
        match result {
            HookScriptResult::Action(HookAction::Block { reason }) => assert_eq!(reason, "no"),
            other => panic!("expected Block action, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_empty_stdout_is_allow() {
        let result = run_hook_script(
            "echo ''",
            &tool_call_event(),
            HookPhase::Pre,
            Some(Duration::from_secs(5)),
            &make_env(),
        )
        .await;
        assert!(matches!(
            result,
            HookScriptResult::Action(HookAction::Allow)
        ));
    }

    #[tokio::test]
    async fn test_no_output_is_allow() {
        let result = run_hook_script(
            "true",
            &tool_call_event(),
            HookPhase::Pre,
            Some(Duration::from_secs(5)),
            &make_env(),
        )
        .await;
        assert!(matches!(
            result,
            HookScriptResult::Action(HookAction::Allow)
        ));
    }

    #[tokio::test]
    async fn test_nonzero_exit_is_failure() {
        let result = run_hook_script(
            "exit 1",
            &tool_call_event(),
            HookPhase::Pre,
            Some(Duration::from_secs(5)),
            &make_env(),
        )
        .await;
        assert!(
            matches!(
                result,
                HookScriptResult::Failed(HookScriptFailure::NonZeroExit { code: Some(1), .. })
            ),
            "non-zero exit should classify as nonzero_exit, got {result:?}"
        );
    }

    #[tokio::test]
    async fn test_timeout_is_failure() {
        let result = run_hook_script(
            "sleep 10",
            &tool_call_event(),
            HookPhase::Pre,
            Some(Duration::from_millis(50)),
            &make_env(),
        )
        .await;
        assert!(
            matches!(
                result,
                HookScriptResult::Failed(HookScriptFailure::Timeout(_))
            ),
            "timeout should classify as timeout, got {result:?}"
        );
    }

    #[tokio::test]
    async fn test_bogus_stdout_is_failure() {
        let result = run_hook_script(
            "echo 'not json'",
            &tool_call_event(),
            HookPhase::Pre,
            Some(Duration::from_secs(5)),
            &make_env(),
        )
        .await;
        assert!(
            matches!(
                result,
                HookScriptResult::Failed(HookScriptFailure::BadJson(_))
            ),
            "invalid stdout should classify as bad_json, got {result:?}"
        );
    }

    #[test]
    fn failure_kinds_are_stable() {
        assert_eq!(
            HookScriptFailure::SpawnFailed("x".into()).kind(),
            "spawn_failed"
        );
        assert_eq!(
            HookScriptFailure::NonZeroExit {
                code: Some(2),
                stderr: "e".into(),
            }
            .kind(),
            "nonzero_exit"
        );
        assert_eq!(HookScriptFailure::BadJson("x".into()).kind(), "bad_json");
        assert_eq!(HookScriptFailure::IoError("x".into()).kind(), "io_error");
        assert_eq!(
            HookScriptFailure::Timeout(Duration::from_millis(5)).kind(),
            "timeout"
        );
    }
}
