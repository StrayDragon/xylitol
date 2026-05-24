use adk_core::{AdkError, ErrorCategory, ErrorComponent, Result, Tool, ToolContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &str {
        "find"
    }

    fn description(&self) -> &str {
        "Find files and directories matching a glob pattern under the given root directory."
    }

    fn parameters_schema(&self) -> Option<Value> {
        Some(json!({
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
                    "find.missing_pattern",
                    "missing required argument: pattern",
                )
            })?;

        let root_path = args.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::InvalidInput,
                "find.missing_path",
                "missing required argument: path",
            )
        })?;

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(100)
            .max(1) as usize;
        let max_results = max_results.min(1000);

        if pattern.starts_with('/') || pattern.starts_with(std::path::MAIN_SEPARATOR) {
            return Err(AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::InvalidInput,
                "find.absolute_pattern_rejected",
                "absolute patterns are not allowed; use a relative pattern within the root path",
            ));
        }

        let root = std::path::Path::new(root_path);
        if !root.exists() {
            return Err(AdkError::new(
                ErrorComponent::Tool,
                ErrorCategory::NotFound,
                "find.root_not_found",
                format!("root path does not exist: '{}'", root_path),
            ));
        }

        let full_pattern = {
            let joined = root.join(pattern);
            joined.to_string_lossy().to_string()
        };

        let mut files = Vec::new();
        match glob::glob(&full_pattern) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(path) => {
                            files.push(path.to_string_lossy().to_string());
                            if files.len() >= max_results {
                                break;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Err(e) => {
                return Err(AdkError::new(
                    ErrorComponent::Tool,
                    ErrorCategory::InvalidInput,
                    "find.invalid_pattern",
                    format!("invalid glob pattern '{}': {}", pattern, e),
                ));
            }
        }

        Ok(json!({
            "files": files,
            "total": files.len(),
            "pattern": pattern,
            "path": root_path,
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
                test_context(),
                json!({
                    "pattern": "**/*.rs",
                    "path": dir.path().to_str().unwrap(),
                }),
            )
            .await
            .unwrap();

        let files: Vec<&str> = result["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                // Extract just filename for comparison
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
                test_context(),
                json!({
                    "pattern": "*.rs",
                    "path": "/nonexistent_root_12345",
                }),
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
                test_context(),
                json!({
                    "pattern": "*.nonexistent_ext",
                    "path": dir.path().to_str().unwrap(),
                }),
            )
            .await
            .unwrap();

        assert_eq!(result["total"], 0);
    }

    #[tokio::test]
    async fn test_find_missing_args() {
        let tool = FindTool;
        assert!(tool.execute(test_context(), json!({})).await.is_err());
        assert!(
            tool.execute(test_context(), json!({ "pattern": "*.rs" }))
                .await
                .is_err()
        );
    }
}
