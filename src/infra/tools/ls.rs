//! Ls tool — lists directory contents.
//!
//! Key behaviors:
//! - Sorted alphabetically (case-insensitive)
//! - Directories suffixed with `/`
//! - Optional path (defaults to cwd)
//! - Configurable limit (default 200)
//! - Entry limit hint when truncated

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use super::path_utils::resolve_to_dir;
use super::typed::TypedTool;
use crate::protocol::error::XyToolError;
use crate::protocol::ports::XyToolCtx;

const DEFAULT_LS_LIMIT: usize = 200;

pub struct LsTool;

#[derive(Debug, Deserialize)]
pub struct LsArgs {
    #[serde(default = "default_ls_path")]
    path: String,
    #[serde(default = "default_ls_limit")]
    limit: u64,
}

fn default_ls_path() -> String {
    ".".into()
}

fn default_ls_limit() -> u64 {
    DEFAULT_LS_LIMIT as u64
}

#[async_trait]
impl TypedTool for LsTool {
    type Args = LsArgs;

    fn name(&self) -> &str {
        "ls"
    }

    fn description(&self) -> &str {
        "List files and directories at the given path. Directories are suffixed with '/'. Results are sorted alphabetically."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (default: current directory)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of entries to return (default: 200)"
                }
            },
            "required": []
        })
    }
    fn wait_bound(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(
            super::typed::FS_TOOL_TIMEOUT_SECS,
        ))
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: LsArgs) -> Result<String, XyToolError> {
        let LsArgs {
            path: dir_path,
            limit,
        } = args;
        let limit = limit as usize;

        let resolved = resolve_to_dir(&ctx.workspace, &dir_path);
        let _resolved_str = resolved.to_string_lossy().to_string();

        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        let mut read_dir = tokio::fs::read_dir(&resolved).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to list '{dir_path}': {e}"))
        })?;

        let mut entries: Vec<(String, bool)> = Vec::new();

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("read entry error: {e}")))?
        {
            let file_type = entry.file_type().await.map_err(|e| {
                XyToolError::ExecutionFailed(anyhow::anyhow!("file type error: {e}"))
            })?;

            let name = entry.file_name().to_string_lossy().to_string();
            entries.push((name, file_type.is_dir()));
        }

        // Sort alphabetically, case-insensitive
        entries.sort_by(|(a_name, a_dir), (b_name, b_dir)| {
            let a_lower = a_name.to_lowercase();
            let b_lower = b_name.to_lowercase();
            a_lower.cmp(&b_lower).then_with(|| {
                // Directories first, then files
                b_dir.cmp(a_dir)
            })
        });

        let mut limit_reached = false;
        let display_entries: Vec<String> = entries
            .iter()
            .take(limit)
            .map(|(name, is_dir)| {
                if *is_dir {
                    format!("{name}/")
                } else {
                    name.clone()
                }
            })
            .collect();

        if entries.len() > limit {
            limit_reached = true;
        }

        if display_entries.is_empty() {
            return Ok("(empty directory)".to_string());
        }

        let mut output = display_entries.join("\n");
        if limit_reached {
            output.push_str(&format!(
                "\n\n[{} entries shown of {} total. Use limit={} for more]",
                limit,
                entries.len(),
                limit * 2
            ));
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::tools::test_ctx;
    use crate::protocol::ports::XyTool;
    use serde_json::json;

    #[tokio::test]
    async fn test_ls_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let tool = LsTool;
        let result = tool
            .execute(&test_ctx(), json!({"path": dir.path().to_str().unwrap()}))
            .await
            .unwrap();
        assert!(result.contains("empty directory"));
    }

    #[tokio::test]
    async fn test_ls_with_files() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.txt"), "")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("b.txt"), "")
            .await
            .unwrap();
        tokio::fs::create_dir(dir.path().join("sub_dir"))
            .await
            .unwrap();

        let tool = LsTool;
        let result = tool
            .execute(&test_ctx(), json!({"path": dir.path().to_str().unwrap()}))
            .await
            .unwrap();
        assert!(result.contains("a.txt"));
        assert!(result.contains("b.txt"));
        assert!(result.contains("sub_dir/"));
    }

    #[tokio::test]
    async fn test_ls_nonexistent_path() {
        let tool = LsTool;
        assert!(
            tool.execute(&test_ctx(), json!({"path": "/nonexistent_12345"}))
                .await
                .is_err()
        );
    }
}
