//! Ls tool — lists directory contents.
//!
//! Key behaviors (aligns with pi's ls.ts):
//! - Sorted alphabetically (case-insensitive)
//! - Directories suffixed with `/`
//! - Optional path (defaults to cwd)
//! - Configurable limit (default 200)
//! - Entry limit hint when truncated

use async_trait::async_trait;
use serde_json::{Value, json};

use super::path_utils::resolve_to_cwd;
use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};

const DEFAULT_LS_LIMIT: usize = 200;

pub struct LsTool;

#[async_trait]
impl XyTool for LsTool {
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

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let dir_path = args["path"].as_str().unwrap_or(".");
        let limit = args["limit"].as_u64().unwrap_or(DEFAULT_LS_LIMIT as u64) as usize;

        let resolved = resolve_to_cwd(dir_path);
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

    fn test_ctx() -> XyToolCtx {
        XyToolCtx::new("test-call")
    }

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
