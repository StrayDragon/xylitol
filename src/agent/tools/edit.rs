use std::path::Path;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};

use super::patch;

pub(crate) struct EditTool;

async fn atomic_write(file_path: &str, content: &str) -> Result<(), XyToolError> {
    let path = Path::new(file_path);
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
    })
}

#[async_trait]
impl XyTool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit a file by finding and replacing text. Uses exact match first, then falls back to fuzzy matching."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn execute(&self, _ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                XyToolError::InvalidArgs("missing required argument: file_path".into())
            })?;

        let old_string = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                XyToolError::InvalidArgs("missing required argument: old_string".into())
            })?;

        let new_string = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                XyToolError::InvalidArgs("missing required argument: new_string".into())
            })?;

        let content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to read '{}': {}", file_path, e))
        })?;

        if let Some(modified) = try_exact_replace(&content, old_string, new_string) {
            let diff = patch::generate_diff(old_string, &modified);
            atomic_write(file_path, &modified).await?;
            return Ok(serde_json::to_string(&json!({
                "success": true,
                "path": file_path,
                "diff": diff,
                "strategy": "exact",
            }))
            .unwrap());
        }

        if let Some(modified) = patch::fudiff_replace(&content, old_string, new_string) {
            let diff = patch::generate_diff(old_string, &modified);
            atomic_write(file_path, &modified).await?;
            return Ok(serde_json::to_string(&json!({
                "success": true,
                "path": file_path,
                "diff": diff,
                "strategy": "fuzzy",
            }))
            .unwrap());
        }

        if let Some(modified) = patch::patch_fallback(&content, old_string, new_string) {
            let diff = patch::generate_diff(old_string, &modified);
            atomic_write(file_path, &modified).await?;
            return Ok(serde_json::to_string(&json!({
                "success": true,
                "path": file_path,
                "diff": diff,
                "strategy": "patch",
            }))
            .unwrap());
        }

        Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
            "could not find '{}...' in '{}'",
            &old_string[..old_string.len().min(50)],
            file_path,
        )))
    }
}

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

    fn test_ctx() -> XyToolCtx {
        XyToolCtx {
            call_id: "test-call".into(),
        }
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
                &test_ctx(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "old_string": "foo bar",
                    "new_string": "baz qux",
                }),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["success"], true);
        assert_eq!(v["strategy"], "exact");

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "hello world\nbaz qux\n");
    }

    #[tokio::test]
    async fn test_edit_nonexistent_file() {
        let tool = EditTool;
        let result = tool
            .execute(
                &test_ctx(),
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
                &test_ctx(),
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
        tokio::fs::write(&path, "hello   world\n").await.unwrap();

        let tool = EditTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({
                    "file_path": path.to_str().unwrap(),
                    "old_string": "hello world",
                    "new_string": "hi world",
                }),
            )
            .await;
        assert!(
            result.is_ok(),
            "fuzzy match should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_edit_missing_args() {
        let tool = EditTool;
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
        assert!(
            tool.execute(&test_ctx(), json!({ "file_path": "/tmp/x" }))
                .await
                .is_err()
        );
        assert!(
            tool.execute(
                &test_ctx(),
                json!({ "file_path": "/tmp/x", "old_string": "a" }),
            )
            .await
            .is_err()
        );
    }
}
