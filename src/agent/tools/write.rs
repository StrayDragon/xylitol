use adk_core::{AdkError, ErrorCategory, ErrorComponent, Result, Tool, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with the given content. Creates parent directories if needed."
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({
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
        }))
    }

    async fn execute(&self, _ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::InvalidInput,
                    "write.missing_path",
                    "missing required argument: file_path",
                )
            })?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::InvalidInput,
                    "write.missing_content",
                    "missing required argument: content",
                )
            })?;

        let path = std::path::Path::new(file_path);

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::Internal,
                    "write.mkdir_failed",
                    format!(
                        "failed to create parent directories for '{}': {}",
                        file_path, e
                    ),
                )
            })?;
        }

        tokio::fs::write(path, content).await.map_err(|e| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::Internal,
                "write.write_failed",
                format!("failed to write '{}': {}", file_path, e),
            )
        })?;

        Ok(json!({
            "success": true,
            "path": file_path,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_context() -> Arc<dyn ToolContext> {
        crate::agent::tools::patch::mock_context()
    }

    #[tokio::test]
    async fn test_write_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_file.txt");

        let tool = WriteTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "content": "hello world",
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["success"], true);

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/sub/dir/file.txt");

        let tool = WriteTool;
        tool.execute(
            test_context(),
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
            test_context(),
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

        let result = tool.execute(test_context(), json!({})).await;
        assert!(result.is_err());

        let result = tool
            .execute(test_context(), json!({ "file_path": "/tmp/x" }))
            .await;
        assert!(result.is_err());
    }
}
