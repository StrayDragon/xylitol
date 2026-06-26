//! Read tool — reads file contents with truncation and offset support.
//!
//! Key behaviors (aligns with pi's read.ts):
//! - Truncation at 2000 lines OR 50KB (whichever first)
//! - Offset (line-based) and limit support
//! - Reports offset-out-of-bounds
//! - Remaining lines hint when truncated
//! - Image file detection: returns placeholder for images

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::core::error::XyToolError;
use crate::core::ports::{XyTool, XyToolCtx};

use super::truncate::{TruncationOptions, truncate_head};

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "ico", "svg"];

pub struct ReadTool;

#[async_trait]
impl XyTool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Supports images and text files with optional offset and limit for large files. Output is truncated to 2000 lines / 50KB."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
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
            "required": ["path"]
        })
    }

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let file_path = args["path"]
            .as_str()
            .ok_or_else(|| XyToolError::InvalidArgs("missing 'path'".into()))?;

        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        let metadata = tokio::fs::metadata(file_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to stat '{file_path}': {e}"))
        })?;

        // Detect image files by extension
        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            // Return placeholder for images (like pi does)
            return Ok(format!(
                "[Image file: {file_path} ({size})]",
                size = format_size(metadata.len())
            ));
        }

        let raw_content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to read '{file_path}': {e}"))
        })?;

        let total_lines = raw_content.lines().count();
        let offset = args["offset"].as_i64().unwrap_or(0);
        let limit = args["limit"].as_i64();

        // Apply offset/limit filtering
        let filtered = if offset <= 0 && limit.is_none() {
            raw_content
        } else {
            let lines: Vec<&str> = raw_content.lines().collect();
            let start = (offset.max(1) as usize).saturating_sub(1);
            let end = match limit {
                Some(n) if n > 0 => (start + n as usize).min(total_lines),
                _ => total_lines,
            };

            if start >= total_lines {
                return Ok(json!({
                    "content": "",
                    "total_lines": total_lines,
                    "offset": offset,
                    "note": "offset exceeds file length"
                })
                .to_string());
            }

            lines[start..end].join("\n")
        };

        // Truncate
        let truncation = truncate_head(&filtered, TruncationOptions::default());

        let mut result = json!({
            "content": truncation.content,
            "total_lines": total_lines,
        });

        if offset > 0 {
            result["offset"] = json!(offset);
        }
        if truncation.truncated {
            let remaining = total_lines.saturating_sub(truncation.output_lines);
            result["truncated"] = json!(true);
            result["truncated_by"] = json!(truncation.truncated_by.map(|l| l.to_string()));
            if remaining > 0 {
                result["remaining_lines"] = json!(remaining);
                result["hint"] = json!(format!(
                    "Output truncated. {} lines remaining. Use offset={} to read more.",
                    remaining,
                    offset.max(1) as usize + truncation.output_lines
                ));
            }
        }

        Ok(serde_json::to_string(&result).expect("serde_json::to_string on Value/Map never fails"))
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> XyToolCtx {
        XyToolCtx::new("test-call")
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
            .execute(&test_ctx(), json!({"path": path.to_str().unwrap()}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["content"], "hello\nworld\nthird line\n");
        assert_eq!(v["total_lines"], 3);
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
                &test_ctx(),
                json!({"path": path.to_str().unwrap(), "offset": 2, "limit": 2}),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["content"], "line2\nline3");
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadTool;
        assert!(
            tool.execute(&test_ctx(), json!({"path": "/nonexistent/read_test_file"}))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_read_missing_path() {
        let tool = ReadTool;
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
    }

    #[tokio::test]
    async fn test_read_offset_past_end() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("short.txt");
        tokio::fs::write(&path, "one line\n").await.unwrap();

        let tool = ReadTool;
        let result = tool
            .execute(
                &test_ctx(),
                json!({"path": path.to_str().unwrap(), "offset": 10}),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["content"], "");
        assert!(v["note"].as_str().unwrap_or("").contains("exceeds"));
    }
}
