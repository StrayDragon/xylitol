//! Edit tool — aligns with pi's edit.ts (multi-edit, original-file matching).
//!
//! Key behaviors:
//! - All edits match against the ORIGINAL file content, not sequentially.
//! - Rejects overlapping edits, non-unique oldText, no-change edits.
//! - Normalizes CRLF/CR → LF.
//! - Strips/restores UTF-8 BOM.
//! - Fuzzy-matches via NFKC normalization of smart quotes and dashes.
//! - Returns unified patch AND display diff with line numbers.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::protocol::error::XyToolError;
use crate::protocol::ports::{XyTool, XyToolCtx};

use super::args::parse_tool_args;
use super::mutation::FileMutationQueue;
use super::patch;

pub struct EditTool {
    mutation_queue: Arc<FileMutationQueue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditItem {
    old_text: String,
    new_text: String,
}

#[derive(Debug, Deserialize)]
struct EditArgs {
    path: String,
    edits: Vec<EditItem>,
}

impl EditTool {
    pub fn new(mutation_queue: Arc<FileMutationQueue>) -> Self {
        Self { mutation_queue }
    }

    fn find_match_range(
        file_content: &str,
        old_text: &str,
        file_path: &str,
    ) -> Result<std::ops::Range<usize>, XyToolError> {
        // Exact match
        if let Some(pos) = file_content.find(old_text) {
            return Ok(pos..pos + old_text.len());
        }
        // Fuzzy
        if let Some(pos) = patch::fuzzy_find(file_content, old_text) {
            return Ok(pos);
        }
        // Patch fallback
        if let Some(pos) = patch::patch_find_range(file_content, old_text) {
            return Ok(pos);
        }
        let snippet = &old_text[..old_text.len().min(50)];
        Err(XyToolError::ExecutionFailed(anyhow::anyhow!(
            "Could not find '{snippet}...' in '{file_path}'"
        )))
    }

    fn check_overlaps(ranges: &[(usize, (usize, usize))]) -> Result<(), XyToolError> {
        for i in 0..ranges.len() {
            for j in (i + 1)..ranges.len() {
                let (_, (as_, ae)) = ranges[i];
                let (_, (bs, be)) = ranges[j];
                if as_ < be && bs < ae {
                    return Err(XyToolError::InvalidArgs(format!(
                        "Overlapping edits: edit {i} and edit {j} overlap"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Core edit logic — isolated for use with mutation queue.
    async fn do_edit(
        file_path: &str,
        edit_pairs: Vec<(String, String)>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<String, XyToolError> {
        if cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        let raw_content = tokio::fs::read_to_string(file_path).await.map_err(|e| {
            XyToolError::ExecutionFailed(anyhow::anyhow!("failed to read '{file_path}': {e}"))
        })?;

        let has_bom = raw_content.starts_with('\u{FEFF}');
        let content_no_bom = if has_bom {
            raw_content[3..].to_string()
        } else {
            raw_content.clone()
        };
        let normalized = content_no_bom.replace("\r\n", "\n").replace('\r', "\n");

        // Match all against ORIGINAL content
        let mut match_ranges: Vec<(usize, (usize, usize))> = Vec::new();
        for (i, (old, _)) in edit_pairs.iter().enumerate() {
            let range = Self::find_match_range(&normalized, old, file_path)?;
            match_ranges.push((i, (range.start, range.end)));
        }
        Self::check_overlaps(&match_ranges)?;

        // Capture matched text per edit
        let mut matched_old_texts: Vec<String> = edit_pairs.iter().map(|_| String::new()).collect();
        for &(i, (start, end)) in &match_ranges {
            matched_old_texts[i] = normalized[start..end].to_string();
        }

        // Build result by applying edits in sorted order
        let mut sorted: Vec<(usize, (usize, usize))> = match_ranges;
        sorted.sort_by_key(|(_, (s, _))| *s);
        let mut result_content = String::new();
        let mut last = 0usize;
        for &(i, (start, end)) in &sorted {
            result_content.push_str(&normalized[last..start]);
            result_content.push_str(&edit_pairs[i].1);
            last = end;
        }
        result_content.push_str(&normalized[last..]);

        let unified = patch::generate_unified_diff(&normalized, &result_content, file_path);
        let display = patch::generate_display_diff(&normalized, &result_content, file_path);

        let final_content = if has_bom {
            format!("\u{FEFF}{result_content}")
        } else {
            result_content
        };

        // Atomic write
        let path = std::path::Path::new(file_path);
        let temp_path = path.with_extension("xylitol-tmp");
        tokio::fs::write(&temp_path, &final_content)
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("write temp: {e}")))?;
        tokio::fs::rename(&temp_path, path).await.map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            XyToolError::ExecutionFailed(anyhow::anyhow!("rename: {e}"))
        })?;

        Ok(serde_json::to_string(&json!({
            "success": true,
            "path": file_path,
            "diff": unified,
            "display_diff": display,
            "strategy": "exact-multi",
            "edit_count": edit_pairs.len(),
        }))
        .expect("serde_json::to_string on Value/Map never fails"))
    }
}

#[async_trait]
impl XyTool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit a file by finding and replacing text in one or more locations. All edits match against the original file content."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "Path to the file to edit"},
                "edits": {
                    "type": "array",
                    "description": "Array of {oldText, newText} pairs to replace",
                    "items": {
                        "type": "object",
                        "properties": {
                            "oldText": {"type": "string", "description": "Text to search for and replace"},
                            "newText": {"type": "string", "description": "Text to replace with"}
                        },
                        "required": ["oldText", "newText"]
                    }
                }
            },
            "required": ["path", "edits"]
        })
    }

    fn execution_mode(&self) -> crate::protocol::ports::XyToolExecutionMode {
        crate::protocol::ports::XyToolExecutionMode::Sequential
    }

    async fn execute(&self, ctx: &XyToolCtx, args: Value) -> Result<String, XyToolError> {
        let EditArgs {
            path: file_path,
            edits,
        } = parse_tool_args(args)?;
        if edits.is_empty() {
            return Err(XyToolError::InvalidArgs("edits must not be empty".into()));
        }

        let edit_pairs: Vec<(String, String)> = edits
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                if e.old_text.is_empty() {
                    return Err(XyToolError::InvalidArgs(format!(
                        "edits[{i}].oldText must not be empty"
                    )));
                }
                if e.old_text == e.new_text {
                    return Err(XyToolError::InvalidArgs(format!(
                        "edits[{i}]: oldText == newText (no change)"
                    )));
                }
                Ok((e.old_text, e.new_text))
            })
            .collect::<Result<_, _>>()?;

        // Check uniqueness
        let mut seen = HashSet::new();
        for (i, (old, _)) in edit_pairs.iter().enumerate() {
            if !seen.insert(old.as_str()) {
                return Err(XyToolError::InvalidArgs(format!(
                    "edits[{i}].oldText is not unique"
                )));
            }
        }

        let cancel = ctx.cancel.clone();
        let fp = file_path;

        // Use mutation queue to serialize same-path edits
        let result_text = self
            .mutation_queue
            .run(&fp, || {
                let edit_pairs = edit_pairs.clone();
                let cancel = cancel.clone();
                let file_path = fp.clone();
                async move {
                    Self::do_edit(&file_path, edit_pairs, cancel)
                        .await
                        .map_err(|e| e.to_string())
                }
            })
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!("{e}")))?;

        Ok(result_text)
    }

    fn prompt_guidelines(&self) -> &[&str] {
        &[
            "Use edit for precise changes (edits[].oldText must match exactly)",
            "When changing multiple separate locations in one file, use one edit call with multiple entries in edits[] instead of multiple edit calls",
            "Each edits[].oldText is matched against the original file, not after earlier edits are applied. Do not emit overlapping or nested edits. Merge nearby changes into one edit.",
            "Keep edits[].oldText as small as possible while still being unique in the file. Do not pad with large unchanged regions.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> XyToolCtx {
        XyToolCtx::new("test-call")
    }

    #[tokio::test]
    async fn test_edit_single_replace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let ps = path.to_str().unwrap().to_string();
        tokio::fs::write(&path, "hello world\nfoo bar\n")
            .await
            .unwrap();

        let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
        let r = tool
            .execute(
                &test_ctx(),
                json!({"path": &ps, "edits": [{"oldText": "foo bar", "newText": "baz qux"}]}),
            )
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["success"], true);
        assert_eq!(
            tokio::fs::read_to_string(&path).await.unwrap(),
            "hello world\nbaz qux\n"
        );
    }

    #[tokio::test]
    async fn test_edit_multiple_non_overlapping() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let ps = path.to_str().unwrap().to_string();
        tokio::fs::write(&path, "AAA\nBBB\nCCC\n").await.unwrap();

        let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
        tool.execute(
            &test_ctx(),
            json!({"path": &ps, "edits": [{"oldText":"AAA","newText":"111"},{"oldText":"CCC","newText":"333"}]}),
        )
        .await
        .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(&path).await.unwrap(),
            "111\nBBB\n333\n"
        );
    }

    #[tokio::test]
    async fn test_edit_rejects_overlap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let ps = path.to_str().unwrap().to_string();
        tokio::fs::write(&path, "ABCDEF\n").await.unwrap();

        let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
        let r = tool
            .execute(
                &test_ctx(),
                json!({"path": &ps, "edits": [{"oldText":"ABC","newText":"111"},{"oldText":"BCD","newText":"222"}]}),
            )
            .await;
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("overlap"));
    }

    #[tokio::test]
    async fn test_edit_rejects_empty_oldtext() {
        let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
        assert!(
            tool.execute(
                &test_ctx(),
                json!({"path":"/tmp/x","edits":[{"oldText":"","newText":"f"}]})
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn test_edit_rejects_no_change() {
        let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
        assert!(
            tool.execute(
                &test_ctx(),
                json!({"path":"/tmp/x","edits":[{"oldText":"s","newText":"s"}]})
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn test_edit_crlf_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let ps = path.to_str().unwrap().to_string();
        tokio::fs::write(&path, "hello\r\nworld\r\n").await.unwrap();

        let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
        let r = tool
            .execute(
                &test_ctx(),
                json!({"path": &ps, "edits": [{"oldText":"hello\nworld","newText":"hi\nthere"}]}),
            )
            .await;
        assert!(r.is_ok());
    }

    #[tokio::test]
    async fn test_edit_bom_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let ps = path.to_str().unwrap().to_string();
        tokio::fs::write(&path, "\u{FEFF}hello world\n")
            .await
            .unwrap();

        let tool = EditTool::new(Arc::new(FileMutationQueue::new()));
        tool.execute(
            &test_ctx(),
            json!({"path": &ps, "edits": [{"oldText":"hello world","newText":"hi world"}]}),
        )
        .await
        .unwrap();
        let c = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(c.starts_with('\u{FEFF}'));
        assert!(c.contains("hi world"));
    }
}
