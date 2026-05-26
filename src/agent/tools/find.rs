use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::error::XyToolError;
use crate::agent::traits::{XyTool, XyToolCtx};

pub(crate) struct FindTool;

#[async_trait]
impl XyTool for FindTool {
    fn name(&self) -> &str {
        "find"
    }

    fn description(&self) -> &str {
        "Find files and directories matching a glob pattern under the given root directory."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match (e.g., '**/*.rs', '*.toml')"
                },
                "path": {
                    "type": "string",
                    "description": "Root directory to search from"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default 100)"
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

        let root_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| XyToolError::InvalidArgs("missing required argument: path".into()))?;

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(100)
            .max(1) as usize;
        let max_results = max_results.min(1000);

        if pattern.starts_with('/') || pattern.starts_with(std::path::MAIN_SEPARATOR) {
            return Err(XyToolError::InvalidArgs(
                "absolute patterns are not allowed; use a relative pattern within the root path"
                    .into(),
            ));
        }

        let root = std::path::Path::new(root_path);
        if !root.exists() {
            return Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
                "root path does not exist: '{}'",
                root_path
            )));
        }

        let full_pattern = root.join(pattern).to_string_lossy().to_string();

        let mut files = Vec::new();
        match glob::glob(&full_pattern) {
            Ok(entries) => {
                for path in entries.flatten() {
                    files.push(path.to_string_lossy().to_string());
                    if files.len() >= max_results {
                        break;
                    }
                }
            }
            Err(e) => {
                return Err(XyToolError::InvalidArgs(format!(
                    "invalid glob pattern '{}': {}",
                    pattern, e
                )));
            }
        }

        Ok(serde_json::to_string(&json!({
            "files": files,
            "total": files.len(),
            "pattern": pattern,
            "path": root_path,
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
    async fn test_find_glob() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.rs"), "").await.unwrap();
        tokio::fs::write(dir.path().join("b.rs"), "").await.unwrap();
        tokio::fs::write(dir.path().join("c.txt"), "")
            .await
            .unwrap();
        tokio::fs::create_dir(dir.path().join("sub")).await.unwrap();
        tokio::fs::write(dir.path().join("sub/d.rs"), "")
            .await
            .unwrap();

        let tool = FindTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({ "pattern": "**/*.rs", "path": dir.path().to_str().unwrap() }),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        let files: Vec<&str> = v["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                let path = std::path::Path::new(f.as_str().unwrap());
                path.file_name().unwrap().to_str().unwrap()
            })
            .collect();
        assert!(files.contains(&"a.rs"));
        assert!(files.contains(&"b.rs"));
        assert!(files.contains(&"d.rs"));
        assert!(!files.contains(&"c.txt"));
    }

    #[tokio::test]
    async fn test_find_nonexistent_root() {
        let tool = FindTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({ "pattern": "*.rs", "path": "/nonexistent_root_12345" }),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        let tool = FindTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({ "pattern": "*.nonexistent_ext", "path": dir.path().to_str().unwrap() }),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["total"], 0);
    }

    #[tokio::test]
    async fn test_find_missing_args() {
        let tool = FindTool;
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
        assert!(
            tool.execute(&test_ctx(), json!({ "pattern": "*.rs" }))
                .await
                .is_err()
        );
    }
}
