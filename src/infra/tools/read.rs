//! Read tool — reads file contents with truncation and offset support.
//!
//! Key behaviors:
//! - Truncation at 2000 lines OR 50KB (whichever first)
//! - Offset (line-based) and limit support
//! - Reports offset-out-of-bounds
//! - Remaining lines hint when truncated
//! - Image files: resize → `AgentPart::Image` (+ short text note); c1155 / t21

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::infra::image::agent_part_from_image_path;
use crate::protocol::error::XyToolError;
use crate::protocol::message::AgentPart;
use crate::protocol::ports::XyToolCtx;
use crate::utils::format_size;

use super::path_utils::resolve_to_dir;
use super::truncate::{TruncationOptions, truncate_head};
use super::typed::TypedTool;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "ico", "svg"];

pub struct ReadTool;

#[derive(Debug, Deserialize)]
pub struct ReadArgs {
    path: String,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    limit: Option<i64>,
}

fn is_image_path(file_path: &str) -> bool {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    IMAGE_EXTENSIONS.contains(&ext.as_str())
}

#[async_trait]
impl TypedTool for ReadTool {
    type Args = ReadArgs;

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
    fn wait_bound(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(
            super::typed::FS_TOOL_TIMEOUT_SECS,
        ))
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: ReadArgs) -> Result<String, XyToolError> {
        let parts = self.execute_as_parts_typed(ctx, args).await?;
        Ok(parts
            .into_iter()
            .filter_map(|p| match p {
                AgentPart::Text { text } => Some(text),
                AgentPart::Image(_) => Some("[image]".into()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }

    async fn execute_as_parts_typed(
        &self,
        ctx: &XyToolCtx,
        args: ReadArgs,
    ) -> Result<Vec<AgentPart>, XyToolError> {
        let ReadArgs {
            path: file_path,
            offset,
            limit,
        } = args;
        let resolved = resolve_to_dir(&ctx.workspace, &file_path);
        let file_path = resolved.to_string_lossy().into_owned();

        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        let metadata = tokio::fs::metadata(&file_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to stat '{file_path}': {e}"))
        })?;

        if is_image_path(&file_path) {
            let path = std::path::Path::new(&file_path);
            return match agent_part_from_image_path(path) {
                Ok(image_part) => {
                    let note = format!(
                        "Read image file [{size}]",
                        size = format_size(metadata.len())
                    );
                    Ok(vec![AgentPart::text(note), image_part])
                }
                Err(e) => Ok(vec![AgentPart::text(format!(
                    "Read image file [{size}]\n{e}",
                    size = format_size(metadata.len())
                ))]),
            };
        }

        let raw_content = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to read '{file_path}': {e}"))
        })?;

        let total_lines = raw_content.lines().count();

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
                return Ok(vec![AgentPart::text(
                    json!({
                        "content": "",
                        "total_lines": total_lines,
                        "offset": offset,
                        "note": "offset exceeds file length"
                    })
                    .to_string(),
                )]);
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

        Ok(vec![AgentPart::text(
            serde_json::to_string(&result).expect("serde_json::to_string on Value/Map never fails"),
        )])
    }

    fn prompt_guidelines(&self) -> &[&str] {
        &["Use read to examine files instead of cat or sed."]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ports::XyTool;
    use image::{ImageBuffer, Rgb};

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

    #[tokio::test]
    async fn test_read_png_yields_image_part() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tiny.png");
        let img: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_fn(8, 8, |_, _| Rgb([10, 20, 30]));
        img.save(&path).unwrap();

        let tool = ReadTool;
        let parts = tool
            .execute_as_parts(&test_ctx(), json!({"path": path.to_str().unwrap()}))
            .await
            .unwrap();
        assert!(
            parts.iter().any(|p| matches!(p, AgentPart::Text { .. })),
            "expected text note"
        );
        let image = parts.iter().find(|p| matches!(p, AgentPart::Image(_)));
        assert!(image.is_some(), "expected Image part");
        if let Some(AgentPart::Image(img)) = image {
            assert!(img.data.as_ref().is_some_and(|d| !d.is_empty()));
            assert!(img.media_type.starts_with("image/"));
        }
    }
}
