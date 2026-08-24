//! Grep tool — uses ripgrep (`rg`) for file content search.
//!
//! Key behaviors:
//! - Uses external `rg` process via tokio::process::Command
//! - Supports: regex, glob, ignoreCase, literal, context lines, result limit
//! - Defaults to 100 results, truncates long lines to GREP_MAX_LINE_LENGTH
//! - Output truncated to DEFAULT_MAX_BYTES
//! - Respects .gitignore (built into ripgrep)

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use super::path_utils::resolve_to_dir;
use super::truncate::{
    DEFAULT_MAX_BYTES, GREP_MAX_LINE_LENGTH, TruncationOptions, truncate_head, truncate_line,
};
use super::typed::TypedTool;
use crate::protocol::error::XyToolError;
use crate::protocol::ports::XyToolCtx;

/// Per-tool wall-clock default when the model omits `timeout` (c2425).
pub(crate) const GREP_TOOL_TIMEOUT_SECS: u64 = 60;

const DEFAULT_LIMIT: usize = 100;
pub struct GrepTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrepArgs {
    pattern: String,
    #[serde(default = "default_dot")]
    path: String,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    ignore_case: bool,
    #[serde(default)]
    literal: bool,
    #[serde(default)]
    context: u32,
    #[serde(default = "default_grep_limit")]
    limit: u64,
    /// Optional timeout in seconds; omitted means the default bound (120s).
    #[serde(default)]
    timeout: Option<i64>,
}

fn default_dot() -> String {
    ".".into()
}

fn default_grep_limit() -> u64 {
    DEFAULT_LIMIT as u64
}

#[async_trait]
impl TypedTool for GrepTool {
    type Args = GrepArgs;

    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search file contents for a pattern using ripgrep. Returns matching lines with file paths and line numbers. Supports regex, glob filtering, case-insensitive, literal mode, context lines, and result limit."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search pattern (regex or literal string)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search (default: current directory)"
                },
                "glob": {
                    "type": "string",
                    "description": "Filter files by glob pattern, e.g. '*.rs' or '**/*.ts'"
                },
                "ignoreCase": {
                    "type": "boolean",
                    "description": "Case-insensitive search (default: false)"
                },
                "literal": {
                    "type": "boolean",
                    "description": "Treat pattern as literal string instead of regex (default: false)"
                },
                "context": {
                    "type": "integer",
                    "description": "Number of lines to show before and after each match (default: 0)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of matches to return (default: 100)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Optional timeout in seconds; the model may request longer runs (e.g. a 10-minute build). Values are clamped to a 600s ceiling. Zero/negative invalid."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute_typed(&self, ctx: &XyToolCtx, args: GrepArgs) -> Result<String, XyToolError> {
        let GrepArgs {
            pattern,
            path: search_path,
            glob,
            ignore_case,
            literal,
            context,
            limit: limit_val,
            timeout: timeout_arg,
        } = args;
        let tool_timeout = super::process::parse_tool_timeout(timeout_arg, GREP_TOOL_TIMEOUT_SECS)?;
        let effective_limit = (limit_val as usize).max(1);

        let search_dir = resolve_to_dir(&ctx.workspace, &search_path);
        let search_dir_str = search_dir.to_string_lossy().to_string();

        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }

        // Build rg args
        let mut rg_args: Vec<String> = vec![
            "--json".to_string(),
            "--line-number".to_string(),
            "--color=never".to_string(),
            "--hidden".to_string(),
        ];
        if ignore_case {
            rg_args.push("--ignore-case".to_string());
        }
        if literal {
            rg_args.push("--fixed-strings".to_string());
        }
        if let Some(ref g) = glob {
            rg_args.push("--glob".to_string());
            rg_args.push(g.clone());
        }
        if context > 0 {
            rg_args.push("-C".to_string());
            rg_args.push(context.to_string());
        }
        rg_args.push("--".to_string());
        rg_args.push(pattern.to_string());
        rg_args.push(search_dir_str);

        let output =
            super::process::run_search_tool("rg", &rg_args, ctx.cancel.clone(), tool_timeout)
                .await?;

        // rg exit code 0 = matches, 1 = no matches, >1 = error
        if !output.status.success() && output.status.code() != Some(1) {
            return Err(super::process::external_tool_failure(&output, "ripgrep"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok("No matches found".to_string());
        }

        // Parse JSON lines
        let mut match_count = 0usize;
        let mut match_limit_reached = false;
        let mut formatted_lines: Vec<String> = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if event["type"] != "match" {
                continue;
            }
            match_count += 1;
            if match_count > effective_limit {
                match_limit_reached = true;
                break;
            }
            let file_path = event["data"]["path"]["text"].as_str().unwrap_or("?");
            let line_num = event["data"]["line_number"].as_u64().unwrap_or(0);
            let match_text = event["data"]["lines"]["text"].as_str().unwrap_or("");
            let sanitized = match_text
                .replace('\r', "")
                .trim_end_matches('\n')
                .to_string();
            let truncated = truncate_line(&sanitized, GREP_MAX_LINE_LENGTH);
            formatted_lines.push(format!("{file_path}:{line_num}: {}", truncated.text));
        }

        let output_text = formatted_lines.join("\n");

        // Byte truncation
        let truncation = truncate_head(
            &output_text,
            TruncationOptions {
                max_lines: Some(usize::MAX),
                max_bytes: Some(DEFAULT_MAX_BYTES),
            },
        );

        let mut final_output = truncation.content;
        let mut notices: Vec<String> = Vec::new();

        if match_limit_reached {
            notices.push(format!(
                "{} matches limit reached. Use limit={} for more, or refine pattern",
                effective_limit,
                effective_limit * 2
            ));
        }
        super::truncate::push_limit_notices(&mut final_output, notices, truncation.truncated);

        Ok(final_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::tools::test_ctx;
    use crate::protocol::ports::XyTool;

    #[tokio::test]
    async fn test_grep_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let ps = path.to_str().unwrap().to_string();
        tokio::fs::write(&path, "apple\nbanana\ncherry\napple pie\n")
            .await
            .unwrap();

        let tool = GrepTool;
        let result = tool
            .execute(&test_ctx(), json!({"pattern": "apple", "path": &ps}))
            .await
            .unwrap();
        assert!(result.contains("apple"));
    }

    #[tokio::test]
    async fn test_grep_missing_pattern() {
        let tool = GrepTool;
        assert!(tool.execute(&test_ctx(), json!({})).await.is_err());
    }
}
