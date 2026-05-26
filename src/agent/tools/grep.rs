use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};

pub(crate) struct GrepTool;

#[async_trait]
impl XyTool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search for a pattern in a file. Returns matching lines with line numbers."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn execute(&self, _ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| XyToolError::InvalidArgs("missing required argument: pattern".into()))?;

        let file_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| XyToolError::InvalidArgs("missing required argument: path".into()))?;

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(100)
            .max(1) as usize;
        let max_results = max_results.min(1000);

        let metadata = tokio::fs::metadata(file_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to read '{}': {}", file_path, e))
        })?;

        if metadata.len() > 10 * 1024 * 1024 {
            return Err(XyToolError::InvalidArgs(format!(
                "file '{}' is {} bytes, exceeding 10MB limit for grep",
                file_path,
                metadata.len()
            )));
        }

        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to read '{}': {}", file_path, e))
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

        Ok(serde_json::to_string(&json!({
            "matches": matches,
            "total": matches.len(),
            "pattern": pattern,
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
    async fn test_grep_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "apple\nbanana\ncherry\napple pie\norange\n")
            .await
            .unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({ "pattern": "apple", "path": path.to_str().unwrap() }),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["total"], 2);
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, "foo\nbar\nbaz\n").await.unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({ "pattern": "nonexistent", "path": path.to_str().unwrap() }),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["total"], 0);
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
                &test_ctx(),
                json!({ "pattern": "match", "path": path.to_str().unwrap(), "max_results": 3 }),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["total"], 3);
    }

    #[tokio::test]
    async fn test_grep_nonexistent_file() {
        let tool = GrepTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({ "pattern": "test", "path": "/nonexistent/grep_test_file" }),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grep_missing_args() {
        let tool = GrepTool;
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
        assert!(
            tool.execute(&test_ctx(), json!({ "pattern": "x" }))
                .await
                .is_err()
        );
    }
}
