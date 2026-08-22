//! Find tool — uses `fd` to search for files by glob pattern.
//!
//! Key behaviors:
//! - Uses external `fd` process via tokio::process::Command
//! - Respects .gitignore via fd's default behavior
//! - Returns Posix-relative paths
//! - Supports result limit (default 1000)
//! - Supports `--full-path` mode for path-containing patterns

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::process::Command;

use super::path_utils::resolve_to_dir;
use super::truncate::{DEFAULT_MAX_BYTES, TruncationOptions, truncate_head};
use super::typed::TypedTool;
use crate::protocol::error::XyToolError;
use crate::protocol::ports::XyToolCtx;
use crate::protocol::{ToolTimeout, ToolTimeoutError};

/// Per-tool wall-clock default when the model omits `timeout` (c2425).
pub(crate) const FIND_TOOL_TIMEOUT_SECS: u64 = 60;
use crate::utils::format_size;

const DEFAULT_LIMIT: usize = 1000;
pub struct FindTool;

#[derive(Debug, Deserialize)]
pub struct FindArgs {
    pattern: String,
    #[serde(default = "default_find_path")]
    path: String,
    #[serde(default = "default_find_limit")]
    limit: u64,
    /// Optional timeout in seconds; omitted means the default bound (120s).
    #[serde(default)]
    timeout: Option<i64>,
}

fn default_find_path() -> String {
    ".".into()
}

fn default_find_limit() -> u64 {
    DEFAULT_LIMIT as u64
}

#[async_trait]
impl TypedTool for FindTool {
    type Args = FindArgs;

    fn name(&self) -> &str {
        "find"
    }

    fn description(&self) -> &str {
        "Search for files by glob pattern using fd. Returns matching file paths relative to the search directory. Respects .gitignore."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match files, e.g. '*.ts', '**/*.json'"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: current directory)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 1000)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Optional timeout in seconds; the model may request longer runs (e.g. a 10-minute build). Values are clamped to a 600s ceiling. Zero/negative invalid."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: FindArgs) -> Result<String, XyToolError> {
        let FindArgs {
            pattern,
            path: search_path,
            limit,
            timeout: timeout_arg,
        } = args;
        let tool_timeout = ToolTimeout::from_i64_opt(timeout_arg).map_err(|e| match e {
            ToolTimeoutError::ZeroOrNegative => XyToolError::InvalidArgs(e.to_string()),
        })?;
        let tool_timeout = tool_timeout.or_default(FIND_TOOL_TIMEOUT_SECS).clamped();
        let effective_limit = (limit as usize).clamp(1, 10_000);

        let search_dir = resolve_to_dir(&ctx.workspace, &search_path);
        let search_dir_str = search_dir.to_string_lossy().to_string();

        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        // Build fd args
        let mut fd_args: Vec<String> = vec![
            "--glob".to_string(),
            "--color=never".to_string(),
            "--hidden".to_string(),
            "--no-require-git".to_string(),
            "--max-results".to_string(),
            effective_limit.to_string(),
        ];

        let mut effective_pattern = pattern.clone();
        if pattern.contains('/') {
            fd_args.push("--full-path".to_string());
            if !pattern.starts_with('/') && !pattern.starts_with("**/") && pattern != "**" {
                effective_pattern = format!("**/{pattern}");
            }
        }

        fd_args.push("--".to_string());
        fd_args.push(effective_pattern);
        fd_args.push(search_dir_str.clone());

        let cancel = ctx.cancel.clone();
        let child = Command::new("fd")
            .args(&fd_args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("spawn fd: {e}")))?;

        let pid = child.id().unwrap_or(0);

        let deadline = tool_timeout
            .duration()
            .map(|d| tokio::time::Instant::now() + d);
        let child_result = tokio::select! {
            _ = cancel.cancelled() => {
                super::process::kill_tree(pid).await;
                return Err(XyToolError::Aborted);
            }
            r = child.wait_with_output() => r,
            _ = async {
                match deadline {
                    Some(dl) => tokio::time::sleep_until(dl).await,
                    None => std::future::pending::<()>().await,
                }
            } => {
                super::process::kill_tree(pid).await;
                return Err(XyToolError::Timeout(
                    tool_timeout
                        .duration()
                        .expect("timeout arm only fires when limited"),
                ));
            }
        };

        let output = child_result
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("failed to run fd: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = if !stderr.is_empty() {
                stderr.to_string()
            } else {
                format!("fd exited with {}", output.status)
            };
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!("{msg}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok("No files found matching pattern".to_string());
        }

        // Relativize against search root
        let mut relativized: Vec<String> = Vec::new();
        for raw_line in stdout.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            let had_trailing_slash = line.ends_with('/') || line.ends_with('\\');
            let rel = if line.starts_with(&search_dir_str) {
                let rest = &line[search_dir_str.len()..];
                rest.trim_start_matches(std::path::MAIN_SEPARATOR)
                    .to_string()
            } else {
                line.to_string()
            };
            let rel = rel.replace('\\', "/");
            let rel = if had_trailing_slash && !rel.ends_with('/') {
                format!("{rel}/")
            } else {
                rel
            };
            relativized.push(rel);
        }

        let result_limit_reached = relativized.len() >= effective_limit;
        let raw_output = relativized.join("\n");

        let truncation = truncate_head(
            &raw_output,
            TruncationOptions {
                max_lines: Some(usize::MAX),
                max_bytes: Some(DEFAULT_MAX_BYTES),
            },
        );

        let mut final_output = truncation.content;
        let mut notices: Vec<String> = Vec::new();
        if result_limit_reached {
            notices.push(format!("{effective_limit} results limit reached"));
        }
        if truncation.truncated {
            notices.push(format!(
                "{} limit reached",
                format_size(DEFAULT_MAX_BYTES as u64)
            ));
        }
        if !notices.is_empty() {
            final_output.push_str(&format!("\n\n[{}]", notices.join(". ")));
        }

        Ok(final_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ports::XyTool;
    use serde_json::json;

    fn test_ctx() -> XyToolCtx {
        XyToolCtx::new("test-call")
    }

    #[tokio::test]
    async fn test_find_basic() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.rs"), "").await.unwrap();
        tokio::fs::write(dir.path().join("b.rs"), "").await.unwrap();
        tokio::fs::write(dir.path().join("c.txt"), "")
            .await
            .unwrap();

        let tool = FindTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({"pattern": "*.rs", "path": dir.path().to_str().unwrap()}),
            )
            .await
            .unwrap();
        assert!(result.contains("a.rs"));
        assert!(result.contains("b.rs"));
        assert!(!result.contains("c.txt"));
    }

    #[tokio::test]
    async fn test_find_missing_pattern() {
        let tool = FindTool;
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
    }
}
