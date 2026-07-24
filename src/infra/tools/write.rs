//! Write tool — creates or overwrites files atomically.
//!
//! Key behaviors (aligns with pi's write.ts):
//! - Uses FileMutationQueue for per-path serialization
//! - Creates parent directories automatically
//! - Atomic write via temp file + rename
//! - Supports CancellationToken abort

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::protocol::error::XyToolError;
use crate::protocol::ports::XyToolCtx;

use super::mutation::FileMutationQueue;
use super::typed::TypedTool;

pub struct WriteTool {
    mutation_queue: Arc<FileMutationQueue>,
}

#[derive(Debug, Deserialize)]
pub struct WriteArgs {
    path: String,
    content: String,
}

impl WriteTool {
    pub fn new(mutation_queue: Arc<FileMutationQueue>) -> Self {
        Self { mutation_queue }
    }
}

#[async_trait]
impl TypedTool for WriteTool {
    type Args = WriteArgs;

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
                "path": {
                    "type": "string",
                    "description": "Path where to write the file"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn execution_mode(&self) -> crate::protocol::ports::XyToolExecutionMode {
        crate::protocol::ports::XyToolExecutionMode::Sequential
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: WriteArgs) -> Result<String, XyToolError> {
        let WriteArgs {
            path: file_path,
            content,
        } = args;

        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        // Run atomic write under mutation queue
        let mq = self.mutation_queue.clone();
        let fp = file_path;
        let cancel = ctx.cancel.clone();

        mq.run(&fp, || {
            let content = content.clone();
            let cancel = cancel.clone();
            let fp = fp.clone();
            async move {
                if cancel.is_cancelled() {
                    return Err("aborted".to_string());
                }

                let path = std::path::Path::new(&fp);

                // Create parent directories
                if let Some(parent) = path.parent()
                    && !parent.as_os_str().is_empty()
                {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| format!("create parent dirs: {e}"))?;
                }

                // Atomic write
                let temp_path = path.with_extension("xylitol-tmp");
                tokio::fs::write(&temp_path, &content)
                    .await
                    .map_err(|e| format!("write temp: {e}"))?;
                tokio::fs::rename(&temp_path, path).await.map_err(|e| {
                    let _ = std::fs::remove_file(&temp_path);
                    format!("rename: {e}")
                })?;

                Ok(serde_json::to_string(&json!({
                    "success": true,
                    "path": fp,
                    "bytes": content.len(),
                }))
                .expect("serde_json::to_string on Value/Map never fails"))
            }
        })
        .await
        .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("{e}")))
    }

    fn prompt_guidelines(&self) -> &[&str] {
        &["Use write only for new files or complete rewrites."]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ports::XyTool;

    fn test_ctx() -> XyToolCtx {
        XyToolCtx::new("test-call")
    }

    #[tokio::test]
    async fn test_write_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_file.txt");
        let ps = path.to_str().unwrap().to_string();

        let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
        let r = tool
            .execute(&test_ctx(), json!({"path": &ps, "content": "hello world"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["success"], true);
        assert_eq!(
            tokio::fs::read_to_string(&path).await.unwrap(),
            "hello world"
        );
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/sub/dir/file.txt");
        let ps = path.to_str().unwrap().to_string();

        let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
        tool.execute(&test_ctx(), json!({"path": &ps, "content": "nested"}))
            .await
            .unwrap();
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_write_missing_args() {
        let tool = WriteTool::new(Arc::new(FileMutationQueue::new()));
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
        assert!(
            tool.execute(&test_ctx(), json!({"path": "/tmp/x"}))
                .await
                .is_err()
        );
    }
}
