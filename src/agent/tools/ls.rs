use adk_core::{AdkError, ErrorCategory, ErrorComponent, Result, Tool, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &str {
        "ls"
    }

    fn description(&self) -> &str {
        "List files and directories at the given path."
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list"
                }
            },
            "required": ["path"]
        }))
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn execute(&self, _ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
        let dir_path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::InvalidInput,
                "ls.missing_path",
                "missing required argument: path",
            )
        })?;

        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(dir_path).await.map_err(|e| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::NotFound,
                "ls.read_dir_failed",
                format!("failed to list '{}': {}", dir_path, e),
            )
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::Internal,
                "ls.read_entry_failed",
                format!("failed to read entry in '{}': {}", dir_path, e),
            )
        })? {
            let file_type = entry.file_type().await.map_err(|e| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::Internal,
                    "ls.file_type_failed",
                    format!(
                        "failed to get file type for '{}': {}",
                        entry.path().display(),
                        e
                    ),
                )
            })?;

            let name = entry.file_name().to_string_lossy().to_string();
            let entry_type = if file_type.is_dir() {
                "directory"
            } else if file_type.is_symlink() {
                "symlink"
            } else {
                "file"
            };

            entries.push(json!({
                "name": name,
                "type": entry_type,
            }));
        }

        Ok(json!({
            "entries": entries,
            "total": entries.len(),
            "path": dir_path,
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
    async fn test_ls_empty_directory() {
        let dir = tempfile::tempdir().unwrap();

        let tool = LsTool;
        let result = tool
            .execute(
                test_context(),
                json!({ "path": dir.path().to_str().unwrap() }),
            )
            .await
            .unwrap();

        assert_eq!(result["total"], 0);
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
            .execute(
                test_context(),
                json!({ "path": dir.path().to_str().unwrap() }),
            )
            .await
            .unwrap();

        assert_eq!(result["total"], 3);

        let names: Vec<&str> = result["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));
        assert!(names.contains(&"sub"));
    }

    #[tokio::test]
    async fn test_ls_nonexistent_path() {
        let tool = LsTool;
        let result = tool
            .execute(test_context(), json!({ "path": "/nonexistent_path_12345" }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ls_missing_path() {
        let tool = LsTool;
        let result = tool.execute(test_context(), json!({})).await;
        assert!(result.is_err());
    }
}
