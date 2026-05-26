use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};

pub(crate) struct LsTool;

#[async_trait]
impl XyTool for LsTool {
    fn name(&self) -> &str {
        "ls"
    }

    fn description(&self) -> &str {
        "List files and directories at the given path."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, _ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let dir_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| XyToolError::InvalidArgs("missing required argument: path".into()))?;

        let max_entries: usize = 1000;
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(dir_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to list '{}': {}", dir_path, e))
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!(
                "failed to read entry in '{}': {}",
                dir_path,
                e
            ))
        })? {
            let file_type = entry.file_type().await.map_err(|e| {
                XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "failed to get file type for '{}': {}",
                    entry.path().display(),
                    e
                ))
            })?;

            let name = entry.file_name().to_string_lossy().to_string();
            let entry_type = if file_type.is_dir() {
                "directory"
            } else if file_type.is_symlink() {
                "symlink"
            } else {
                "file"
            };

            entries.push(json!({ "name": name, "type": entry_type }));
            if entries.len() >= max_entries {
                break;
            }
        }

        Ok(serde_json::to_string(&json!({
            "entries": entries,
            "total": entries.len(),
            "path": dir_path,
        }))
        .unwrap())
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
    async fn test_ls_empty_directory() {
        let dir = tempfile::tempdir().unwrap();

        let tool = LsTool;
        let result = tool
            .execute(&test_ctx(), json!({ "path": dir.path().to_str().unwrap() }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["total"], 0);
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
        tokio::fs::create_dir(dir.path().join("sub")).await.unwrap();

        let tool = LsTool;
        let result = tool
            .execute(&test_ctx(), json!({ "path": dir.path().to_str().unwrap() }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["total"], 3);
    }

    #[tokio::test]
    async fn test_ls_nonexistent_path() {
        let tool = LsTool;
        let result = tool
            .execute(&test_ctx(), json!({ "path": "/nonexistent_path_12345" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ls_missing_path() {
        let tool = LsTool;
        let result = tool.execute(&test_ctx(), json!({})).await;
        assert!(result.is_err());
    }
}
