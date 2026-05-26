use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};

pub(crate) struct WriteTool;

#[async_trait]
impl XyTool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with the given content. Creates parent directories if needed."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path where to write the file"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        })
    }

    async fn execute(&self, _ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                XyToolError::InvalidArgs("missing required argument: file_path".into())
            })?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| XyToolError::InvalidArgs("missing required argument: content".into()))?;

        let path = std::path::Path::new(file_path);

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "failed to create parent directories for '{}': {}",
                    file_path,
                    e
                ))
            })?;
        }

        let temp_path = path.with_extension("xylitol-tmp");
        tokio::fs::write(&temp_path, content).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!(
                "failed to write temp file for '{}': {}",
                file_path,
                e
            ))
        })?;
        tokio::fs::rename(&temp_path, path).await.map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            XyToolError::ExecutionFailed(anyhow::anyhow!(
                "failed to atomically replace '{}': {}",
                file_path,
                e
            ))
        })?;

        Ok(serde_json::to_string(&json!({
            "success": true,
            "path": file_path,
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
    async fn test_write_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_file.txt");

        let tool = WriteTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "content": "hello world",
                }),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["success"], true);

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/sub/dir/file.txt");

        let tool = WriteTool;
        tool.execute(
            &test_ctx(),
            json!({
                "file_path": path.to_str().unwrap(),
                "content": "nested content",
            }),
        )
        .await
        .unwrap();

        assert!(path.exists());
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "nested content");
    }

    #[tokio::test]
    async fn test_write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("overwrite.txt");
        tokio::fs::write(&path, "old content").await.unwrap();

        let tool = WriteTool;
        tool.execute(
            &test_ctx(),
            json!({
                "file_path": path.to_str().unwrap(),
                "content": "new content",
            }),
        )
        .await
        .unwrap();

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "new content");
    }

    #[tokio::test]
    async fn test_write_missing_args() {
        let tool = WriteTool;

        let result = tool.execute(&test_ctx(), json!({})).await;
        assert!(result.is_err());

        let result = tool
            .execute(&test_ctx(), json!({ "file_path": "/tmp/x" }))
            .await;
        assert!(result.is_err());
    }
}
