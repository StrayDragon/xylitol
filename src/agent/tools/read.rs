use adk_core::{AdkError, ErrorCategory, ErrorComponent, Result, Tool, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// Maximum file size for read operations (10 MB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

pub(crate) struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file from the filesystem. Supports line offset and limit."
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-based)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read"
                }
            },
            "required": ["file_path"]
        }))
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn execute(&self, _ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::InvalidInput,
                    "read.missing_path",
                    "missing required argument: file_path",
                )
            })?;

        let offset = args.get("offset").and_then(|v| v.as_i64()).unwrap_or(0);
        let limit = args.get("limit").and_then(|v| v.as_i64());

        let metadata = tokio::fs::metadata(file_path).await.map_err(|e| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::NotFound,
                "read.read_error",
                format!("failed to read '{}': {}", file_path, e),
            )
        })?;

        if metadata.len() > MAX_FILE_SIZE {
            return Err(AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::InvalidInput,
                "read.file_too_large",
                format!(
                    "file '{}' is {} bytes, exceeding limit of {} bytes. Use offset/limit.",
                    file_path,
                    metadata.len(),
                    MAX_FILE_SIZE
                ),
            ));
        }

        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::NotFound,
                "read.read_error",
                format!("failed to read '{}': {}", file_path, e),
            )
        })?;

        if offset <= 0 && limit.is_none() {
            let line_count = content.lines().count();
            return Ok(json!({
                "content": content,
                "line_count": line_count,
            }));
        }

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let start = (offset.max(1) as usize).saturating_sub(1);
        let end = match limit {
            Some(n) => (start + n as usize).min(total_lines),
            None => total_lines,
        };

        if start >= total_lines {
            return Ok(json!({
                "content": "",
                "line_count": 0,
                "total_lines": total_lines,
                "offset": offset,
            }));
        }

        let excerpt = lines[start..end].join("\n");
        Ok(json!({
            "content": excerpt,
            "line_count": end - start,
            "total_lines": total_lines,
            "offset": offset,
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
    async fn test_read_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "hello\nworld\nthird line\n")
            .await
            .unwrap();

        let tool = ReadTool;
        let result = tool
            .execute(
                test_context(),
                json!({ "file_path": path.to_str().unwrap() }),
            )
            .await
            .unwrap();

        assert_eq!(result["content"], "hello\nworld\nthird line\n");
        assert_eq!(result["line_count"], 3);
    }

    #[tokio::test]
    async fn test_read_with_offset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "line1\nline2\nline3\nline4\n")
            .await
            .unwrap();

        let tool = ReadTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "offset": 2,
                    "limit": 2,
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["content"], "line2\nline3");
        assert_eq!(result["line_count"], 2);
        assert_eq!(result["total_lines"], 4);
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadTool;
        let result = tool
            .execute(
                test_context(),
                json!({ "file_path": "/nonexistent/path/file.txt" }),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_missing_path_arg() {
        let tool = ReadTool;
        let result = tool.execute(test_context(), json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_offset_beyond_end() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("short.txt");
        tokio::fs::write(&path, "only one line\n").await.unwrap();

        let tool = ReadTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "offset": 10,
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["content"], "");
        assert_eq!(result["line_count"], 0);
    }
}
