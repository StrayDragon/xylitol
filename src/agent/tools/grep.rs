use adk_core::{AdkError, ErrorCategory, ErrorComponent, Result, Tool, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a pattern in a file. Returns matching lines with line numbers."
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Text pattern to search for (plain text, not regex)"
                },
                "path": {
                    "type": "string",
                    "description": "Path to the file to search in"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of matches to return (default 100)"
                }
            },
            "required": ["pattern", "path"]
        }))
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn execute(&self, _ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::InvalidInput,
                    "grep.missing_pattern",
                    "missing required argument: pattern",
                )
            })?;

        let file_path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::InvalidInput,
                "grep.missing_path",
                "missing required argument: path",
            )
        })?;

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(100) as usize;

        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::NotFound,
                "grep.read_error",
                format!("failed to read '{}': {}", file_path, e),
            )
        })?;

        let mut matches = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if line.contains(pattern) {
                matches.push(json!({
                    "line": line_num + 1,
                    "content": line,
                }));
                if matches.len() >= max_results {
                    break;
                }
            }
        }

        Ok(json!({
            "matches": matches,
            "total": matches.len(),
            "pattern": pattern,
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
    async fn test_grep_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "apple\nbanana\ncherry\napple pie\norange\n")
            .await
            .unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "pattern": "apple",
                    "path": path.to_str().unwrap(),
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["total"], 2);
        let lines: Vec<usize> = result["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["line"].as_i64().unwrap() as usize)
            .collect();
        assert_eq!(lines, vec![1, 4]);
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "foo\nbar\nbaz\n").await.unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "pattern": "nonexistent",
                    "path": path.to_str().unwrap(),
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["total"], 0);
    }

    #[tokio::test]
    async fn test_grep_max_results() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let content = (0..10)
            .map(|i| format!("line {} match", i))
            .collect::<Vec<_>>()
            .join("\n");
        tokio::fs::write(&path, &content).await.unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "pattern": "match",
                    "path": path.to_str().unwrap(),
                    "max_results": 3,
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["total"], 3);
    }

    #[tokio::test]
    async fn test_grep_nonexistent_file() {
        let tool = GrepTool;
        let result = tool
            .execute(
                test_context(),
                json!({
                    "pattern": "test",
                    "path": "/nonexistent/grep_test_file",
                }),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grep_missing_args() {
        let tool = GrepTool;
        assert!(tool.execute(test_context(), json!({})).await.is_err());
        assert!(
            tool.execute(test_context(), json!({ "pattern": "x" }))
                .await
                .is_err()
        );
    }
}
