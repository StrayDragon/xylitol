//! Find tool — uses `fd` to search for files by glob pattern.
//!
//! Key behaviors (aligns with pi's find.ts):
//! - Uses external `fd` process via tokio::process::Command
//! - Respects .gitignore via fd's default behavior
//! - Returns Posix-relative paths
//! - Supports result limit (default 1000)
//! - Supports `--full-path` mode for path-containing patterns

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

use super::path_utils::resolve_to_cwd;
use super::truncate::{DEFAULT_MAX_BYTES, TruncationOptions, format_size, truncate_head};
use crate::core::error::XyToolError;
use crate::core::ports::{XyTool, XyToolCtx};

const DEFAULT_LIMIT: usize = 1000;
const FD_TIMEOUT: Duration = Duration::from_secs(30);

pub struct FindTool;

#[async_trait]
impl XyTool for FindTool {
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
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| XyToolError::InvalidArgs("missing 'pattern'".into()))?;
        let search_path = args["path"].as_str().unwrap_or(".");
        let limit = args["limit"].as_u64().unwrap_or(DEFAULT_LIMIT as u64) as usize;
        let effective_limit = limit.clamp(1, 10_000);

        let search_dir = resolve_to_cwd(search_path);
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

        let mut effective_pattern = pattern.to_string();
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

        let child_result = tokio::select! {
            _ = cancel.cancelled() => {
                super::process::kill_tree(pid).await;
                return Err(XyToolError::Aborted);
            }
            r = child.wait_with_output() => r,
            _ = timeout(FD_TIMEOUT, std::future::pending::<()>()) => {
                super::process::kill_tree(pid).await;
                return Err(XyToolError::Timeout(FD_TIMEOUT));
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
            notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
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
