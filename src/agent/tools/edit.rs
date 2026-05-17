use adk_core::{AdkError, ErrorCategory, ErrorComponent, Result, Tool, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

use super::patch;

pub(crate) struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit a file by finding and replacing text. Uses exact match first, then falls back to fuzzy matching."
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "Text to search for and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "Text to replace with"
                },
                "coords": {
                    "type": "object",
                    "description": "Explicit selection coordinates",
                    "properties": {
                        "start_line": { "type": "integer" },
                        "end_line": { "type": "integer" }
                    }
                }
            },
            "required": ["file_path", "old_string", "new_string"]
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
                    "edit.missing_path",
                    "missing required argument: file_path",
                )
            })?;

        let old_string = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::InvalidInput,
                    "edit.missing_old",
                    "missing required argument: old_string",
                )
            })?;

        let new_string = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::InvalidInput,
                    "edit.missing_new",
                    "missing required argument: new_string",
                )
            })?;

        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::NotFound,
                "edit.read_error",
                format!("failed to read '{}': {}", file_path, e),
            )
        })?;

        // Step 1: Try exact match first
        if let Some(modified) = try_exact_replace(&content, old_string, new_string) {
            let diff = patch::generate_diff(old_string, &modified);
            tokio::fs::write(file_path, &modified).await.map_err(|e| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::Internal,
                    "edit.write_failed",
                    format!("failed to write '{}': {}", file_path, e),
                )
            })?;

            return Ok(json!({
                "success": true,
                "path": file_path,
                "diff": diff,
                "strategy": "exact",
            }));
        }

        // Step 2: Try fuzzy match (fudiff)
        if let Some(modified) = patch::fudiff_replace(&content, old_string, new_string) {
            let diff = patch::generate_diff(old_string, &modified);
            tokio::fs::write(file_path, &modified).await.map_err(|e| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::Internal,
                    "edit.write_failed",
                    format!("failed to write '{}': {}", file_path, e),
                )
            })?;

            return Ok(json!({
                "success": true,
                "path": file_path,
                "diff": diff,
                "strategy": "fuzzy",
            }));
        }

        // Step 3: Try patch fallback
        if let Some(modified) = patch::patch_fallback(&content, old_string, new_string) {
            let diff = patch::generate_diff(old_string, &modified);
            tokio::fs::write(file_path, &modified).await.map_err(|e| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::Internal,
                    "edit.write_failed",
                    format!("failed to write '{}': {}", file_path, e),
                )
            })?;

            return Ok(json!({
                "success": true,
                "path": file_path,
                "diff": diff,
                "strategy": "patch",
            }));
        }

        Err(AdkError::new(
            ErrorComponent::Tool,
            ErrorCategory::NotFound,
            "edit.pattern_not_found",
            format!(
                "could not find '{}...' in '{}'",
                &old_string[..old_string.len().min(50)],
                file_path,
            ),
        ))
    }
}

/// Try to replace an exact occurrence of `old_string` in `content`.
fn try_exact_replace(content: &str, old_string: &str, new_string: &str) -> Option<String> {
    if content.contains(old_string) {
        Some(content.replace(old_string, new_string))
    } else {
        None
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
    async fn test_edit_exact_replace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "hello world\nfoo bar\n")
            .await
            .unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "old_string": "foo bar",
                    "new_string": "baz qux",
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["strategy"], "exact");

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "hello world\nbaz qux\n");
    }

    #[tokio::test]
    async fn test_edit_nonexistent_file() {
        let tool = EditTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "file_path": "/nonexistent/edit_test_file",
                    "old_string": "foo",
                    "new_string": "bar",
                }),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_edit_pattern_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "hello world\n").await.unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "old_string": "nonexistent text here",
                    "new_string": "replacement",
                }),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_edit_fuzzy_replace_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        // Actual file has extra spaces
        tokio::fs::write(&path, "hello   world\n").await.unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "old_string": "hello world",
                    "new_string": "hi world",
                }),
            )
            .await;

        // Should succeed via fuzzy match
        assert!(
            result.is_ok(),
            "fuzzy match should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_edit_missing_args() {
        let tool = EditTool;
        assert!(tool.execute(test_context(), json!({})).await.is_err());
        assert!(
            tool.execute(test_context(), json!({ "file_path": "/tmp/x" }))
                .await
                .is_err()
        );
        assert!(
            tool.execute(
                test_context(),
                json!({ "file_path": "/tmp/x", "old_string": "a" }),
            )
            .await
            .is_err()
        );
    }
}
